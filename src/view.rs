//! Native view: the HTML single renderer inside a Tauri window.
//!
//! The testable seam is [`prepare_native_view`] plus [`deck_asset`], which
//! serves that HTML and the deck's media. Opening the window is the adapter.

use crate::deck::Deck;
use crate::export;
use std::path::{Path, PathBuf};

/// A deck ready to show in the native window.
#[derive(Debug)]
pub struct NativeView {
    pub title: String,
    pub html: String,
    /// Directory media paths in the HTML are resolved against.
    pub root: PathBuf,
}

/// Bytes returned for one request against the deck.
#[derive(Debug)]
pub struct DeckAsset {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

/// Page the native webview loads. Scheme differs by platform so the custom
/// protocol handler is reached.
pub fn native_index_url() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "http://deck.localhost/index.html"
    } else {
        "deck://localhost/index.html"
    }
}

/// Parse `file` and render it with the shared HTML exporter.
pub fn prepare_native_view(file: &Path) -> Result<NativeView, String> {
    let deck = Deck::from_file(file)?;
    if deck.slides.is_empty() {
        return Err(format!("no slides in {}", file.display()));
    }
    let root = deck_root(file);
    let html = export::export_html(&deck, &root, None, None);
    let title = deck
        .frontmatter
        .title
        .clone()
        .or_else(|| deck.slides.first().and_then(|s| s.title.clone()))
        .unwrap_or_else(|| "keynote".to_string());
    Ok(NativeView { title, html, root })
}

/// Serve the rendered deck or a file under its root. `..` is rejected.
pub fn deck_asset(view: &NativeView, request_path: &str) -> DeckAsset {
    let rel = request_rel(request_path);
    if rel.is_empty() || rel == "index.html" {
        return DeckAsset {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: view.html.as_bytes().to_vec(),
        };
    }
    match safe_join(&view.root, &rel) {
        Ok(path) => match std::fs::read(&path) {
            Ok(body) => DeckAsset {
                status: 200,
                content_type: content_type(&path),
                body,
            },
            Err(_) => missing(),
        },
        Err(JoinErr::Forbidden) => DeckAsset {
            status: 403,
            content_type: "text/plain; charset=utf-8",
            body: b"forbidden".to_vec(),
        },
        Err(JoinErr::Missing) => missing(),
    }
}

/// Open `file` in a native window.
pub fn open(file: &Path) -> Result<(), String> {
    let view = prepare_native_view(file)?;
    open_window(view)
}

#[cfg(feature = "native-view")]
fn open_window(view: NativeView) -> Result<(), String> {
    let title = view.title.clone();
    let root = view.root.clone();
    let html = view.html.clone();
    let url: url::Url = native_index_url()
        .parse()
        .map_err(|e| format!("view url: {e}"))?;
    tauri::Builder::default()
        .register_uri_scheme_protocol("deck", move |_ctx, request| {
            let asset = deck_asset(
                &NativeView {
                    title: String::new(),
                    html: html.clone(),
                    root: root.clone(),
                },
                request.uri().path(),
            );
            tauri::http::Response::builder()
                .status(asset.status)
                .header(tauri::http::header::CONTENT_TYPE, asset.content_type)
                .body(asset.body)
                .unwrap_or_else(|_| {
                    tauri::http::Response::builder()
                        .status(500)
                        .body(Vec::new())
                        .expect("empty response")
                })
        })
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "view", tauri::WebviewUrl::External(url))
                .title(title)
                .inner_size(1280.0, 720.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| e.to_string())
}

#[cfg(not(feature = "native-view"))]
fn open_window(_view: NativeView) -> Result<(), String> {
    Err(
        "native view is compiled out; rebuild with: cargo build --features native-view"
            .into(),
    )
}

fn missing() -> DeckAsset {
    DeckAsset {
        status: 404,
        content_type: "text/plain; charset=utf-8",
        body: b"not found".to_vec(),
    }
}

fn deck_root(file: &Path) -> PathBuf {
    file.parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

enum JoinErr {
    Forbidden,
    Missing,
}

fn request_rel(request_path: &str) -> String {
    let path = request_path.split(['?', '#']).next().unwrap_or(request_path);
    percent_decode(path).trim_start_matches('/').to_string()
}

fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, JoinErr> {
    if rel.contains('\0') {
        return Err(JoinErr::Forbidden);
    }
    let mut out = root.to_path_buf();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return Err(JoinErr::Forbidden);
        }
        out.push(seg);
    }
    if !out.exists() {
        return Err(JoinErr::Missing);
    }
    let root_canon = root.canonicalize().map_err(|_| JoinErr::Missing)?;
    let file_canon = out.canonicalize().map_err(|_| JoinErr::Missing)?;
    if file_canon.starts_with(&root_canon) {
        Ok(file_canon)
    } else {
        Err(JoinErr::Forbidden)
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("mkv") => "video/x-matroska",
        Some("ogv") => "video/ogg",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_deck(dir: &Path, body: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("talk.md");
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn index_url_addresses_the_deck_scheme() {
        let url = native_index_url();
        assert!(url.ends_with("/index.html"), "{url}");
        assert!(url.contains("deck"), "{url}");
    }

    #[test]
    fn prepares_renderer_html_and_deck_title() {
        let dir = std::env::temp_dir().join("keynote-view-prepare");
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_deck(
            &dir,
            "---\ntitle: Native\ntheme: paper\n---\n\n# Hello view\n\n---\n\n## Next\n",
        );
        let view = prepare_native_view(&path).unwrap();
        assert_eq!(view.title, "Native");
        assert!(view.html.contains("Hello view"), "{}", view.html);
        assert!(view.html.contains("class=\"slide\""));
        assert_eq!(view.root, dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_deck_is_an_error() {
        let dir = std::env::temp_dir().join("keynote-view-empty");
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_deck(&dir, "");
        let err = prepare_native_view(&path).unwrap_err();
        assert!(err.contains("no slides"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn serves_index_and_media_and_rejects_escape() {
        let dir = std::env::temp_dir().join("keynote-view-assets");
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_deck(&dir, "---\ntitle: Media\n---\n\n# Slide\n\n![](pic.png)\n");
        std::fs::create_dir_all(dir.join("images")).unwrap();
        std::fs::write(dir.join("images").join("pic.png"), b"PNGDATA").unwrap();
        let view = prepare_native_view(&path).unwrap();

        let index = deck_asset(&view, "/");
        assert_eq!(index.status, 200);
        assert!(index.content_type.starts_with("text/html"));
        assert!(std::str::from_utf8(&index.body).unwrap().contains("Slide"));

        let media = deck_asset(&view, "/images/pic.png");
        assert_eq!(media.status, 200);
        assert_eq!(media.content_type, "image/png");
        assert_eq!(media.body, b"PNGDATA");

        let encoded = deck_asset(&view, "/images/%70ic.png");
        assert_eq!(encoded.status, 200);
        assert_eq!(encoded.body, b"PNGDATA");

        assert_eq!(deck_asset(&view, "/../Cargo.toml").status, 403);
        assert_eq!(deck_asset(&view, "/%2e%2e/Cargo.toml").status, 403);
        assert_eq!(deck_asset(&view, "/images/missing.png").status, 404);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(not(feature = "native-view"))]
    fn open_without_feature_names_the_rebuild() {
        let dir = std::env::temp_dir().join("keynote-view-nofeature");
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_deck(&dir, "# Only\n");
        let err = open(&path).unwrap_err();
        assert!(err.contains("native-view"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
