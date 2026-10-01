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
    /// Markdown source shown in the editor.
    pub source: String,
    /// Directory media paths in the HTML are resolved against.
    pub root: PathBuf,
    pub path: PathBuf,
    /// Shell page (present + editor) served at `app.html`.
    pub shell: String,
    /// Start with the editor pane open.
    pub editor: bool,
}

/// Bytes returned for one request against the deck.
#[derive(Debug)]
pub struct DeckAsset {
    pub status: u16,
    pub content_type: &'static str,
    /// `bytes start-end/total` when this is a partial body.
    pub content_range: Option<String>,
    pub accept_ranges: bool,
    pub body: Vec<u8>,
}

/// Page the native webview loads. Scheme differs by platform so the custom
/// protocol handler is reached.
pub fn native_index_url() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "http://deck.localhost/app.html"
    } else {
        "deck://localhost/app.html"
    }
}

/// Parse `file` and render it with the shared HTML exporter.
pub fn prepare_native_view(file: &Path, editor: bool) -> Result<NativeView, String> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| format!("read {}: {e}", file.display()))?;
    let deck = Deck::from_markdown(&source);
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
    let mut view = NativeView {
        title,
        html,
        source,
        root,
        path: file.to_path_buf(),
        shell: String::new(),
        editor,
    };
    view.shell = editor_page(&view);
    Ok(view)
}

/// Re-render `markdown` with the shared exporter.
pub fn render_markdown(markdown: &str, root: &Path) -> Result<String, String> {
    let deck = Deck::from_markdown(markdown);
    if deck.slides.is_empty() {
        return Err("no slides".into());
    }
    Ok(export::export_html(&deck, root, None, None))
}

/// Write `markdown` back to the deck, keeping a `.bak` of the previous file.
pub fn save_markdown(path: &Path, markdown: &str) -> Result<(), String> {
    if path.exists() {
        crate::backup::backup_file(path);
    }
    std::fs::write(path, markdown).map_err(|e| format!("write {}: {e}", path.display()))
}

/// One slide as the editor shows it: title in the rail, source for the
/// single-slide editor, HTML for the thumbnail and the visual canvas.
#[derive(Debug, serde::Serialize)]
pub struct SlideCard {
    pub title: String,
    pub source: String,
    pub html: String,
}

/// Full deck plus per-slide cards, so Visual, Overview, and Markdown share one render.
#[derive(Debug, serde::Serialize)]
pub struct DeckRender {
    pub markdown: String,
    pub html: String,
    pub slides: Vec<SlideCard>,
}

/// Split a deck into its frontmatter prefix and slide bodies.
pub fn split_editable(markdown: &str) -> Result<(String, Vec<String>), String> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut i = 0usize;
    let mut prefix = String::new();
    if lines.first().map(|l| l.trim() == "---").unwrap_or(false) {
        let mut seen = 0;
        while i < lines.len() {
            if lines[i].trim() == "---" {
                seen += 1;
            }
            prefix.push_str(lines[i]);
            prefix.push('\n');
            i += 1;
            if seen == 2 {
                break;
            }
        }
    }
    let mut slides = Vec::new();
    let mut cur = String::new();
    while i < lines.len() {
        if lines[i].trim() == "---" {
            let text = cur.trim().to_string();
            if !text.is_empty() {
                slides.push(text);
            }
            cur.clear();
            i += 1;
            continue;
        }
        cur.push_str(lines[i]);
        cur.push('\n');
        i += 1;
    }
    let text = cur.trim().to_string();
    if !text.is_empty() {
        slides.push(text);
    }
    if slides.is_empty() {
        return Err("no slides".into());
    }
    Ok((prefix.trim_end().to_string(), slides))
}

pub fn join_editable(prefix: &str, slides: &[String]) -> String {
    let body = slides
        .iter()
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    if prefix.is_empty() {
        format!("{body}\n")
    } else {
        format!("{prefix}\n\n{body}\n")
    }
}

/// Move the slide at `from` so it lands at `to`. Frontmatter stays put.
pub fn reorder_markdown(markdown: &str, from: usize, to: usize) -> Result<String, String> {
    let (prefix, mut slides) = split_editable(markdown)?;
    if from >= slides.len() || to >= slides.len() {
        return Err("slide index out of range".into());
    }
    if from != to {
        let slide = slides.remove(from);
        slides.insert(to, slide);
    }
    Ok(join_editable(&prefix, &slides))
}

/// Replace one slide body. An empty body is refused so the slide is not dropped.
pub fn replace_slide_markdown(
    markdown: &str,
    index: usize,
    slide: &str,
) -> Result<String, String> {
    if slide.trim().is_empty() {
        return Err("a slide needs some text".into());
    }
    let (prefix, mut slides) = split_editable(markdown)?;
    if index >= slides.len() {
        return Err("slide index out of range".into());
    }
    slides[index] = slide.trim().to_string();
    Ok(join_editable(&prefix, &slides))
}

pub fn render_model(markdown: &str, root: &Path) -> Result<DeckRender, String> {
    let deck = Deck::from_markdown(markdown);
    if deck.slides.is_empty() {
        return Err("no slides".into());
    }
    let (_, blocks) = split_editable(markdown)?;
    let html = export::export_html(&deck, root, None, None);
    let slides = deck
        .slides
        .iter()
        .enumerate()
        .map(|(i, s)| SlideCard {
            title: s
                .title
                .clone()
                .unwrap_or_else(|| format!("Slide {}", i + 1)),
            source: blocks.get(i).cloned().unwrap_or_else(|| s.source.clone()),
            html: export::slide_html(&deck, root, i, None, None),
        })
        .collect();
    Ok(DeckRender {
        markdown: markdown.to_string(),
        html,
        slides,
    })
}

fn editor_page(view: &NativeView) -> String {
    let rendered = render_model(&view.source, &view.root).unwrap_or(DeckRender {
        markdown: view.source.clone(),
        html: view.html.clone(),
        slides: Vec::new(),
    });
    #[derive(serde::Serialize)]
    struct Boot<'a> {
        title: &'a str,
        markdown: &'a str,
        html: &'a str,
        editor: bool,
        slides: &'a [SlideCard],
    }
    let boot = serde_json::to_string(&Boot {
        title: &view.title,
        markdown: &view.source,
        html: &rendered.html,
        editor: view.editor,
        slides: &rendered.slides,
    })
    .unwrap_or_else(|_| "{}".into())
    .replace('<', "\\u003c");
    let body_class = if view.editor {
        "mode-visual"
    } else {
        "mode-present"
    };
    include_str!("editor_shell.html")
        .replace("__TITLE__", &html_escape(&view.title))
        .replace("__BODY_CLASS__", body_class)
        .replace("__BOOT_JSON__", &boot)
}


fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Serve the rendered deck or a file under its root. `..` is rejected.
/// `range` is an HTTP `Range` header (`bytes=start-end`); video playback
/// seeks with it.
pub fn deck_asset(view: &NativeView, request_path: &str, range: Option<&str>) -> DeckAsset {
    let rel = request_rel(request_path);
    if rel == "app.html" {
        return DeckAsset {
            status: 200,
            content_type: "text/html; charset=utf-8",
            content_range: None,
            accept_ranges: false,
            body: view.shell.as_bytes().to_vec(),
        };
    }
    if rel.is_empty() || rel == "index.html" {
        return DeckAsset {
            status: 200,
            content_type: "text/html; charset=utf-8",
            content_range: None,
            accept_ranges: false,
            body: view.html.as_bytes().to_vec(),
        };
    }
    match safe_join(&view.root, &rel) {
        Ok(path) => match std::fs::read(&path) {
            Ok(body) => file_asset(content_type(&path), body, range),
            Err(_) => missing(),
        },
        Err(JoinErr::Forbidden) => DeckAsset {
            status: 403,
            content_type: "text/plain; charset=utf-8",
            content_range: None,
            accept_ranges: false,
            body: b"forbidden".to_vec(),
        },
        Err(JoinErr::Missing) => missing(),
    }
}

/// Open `file` in a native window.
pub fn open(file: &Path, editor: bool) -> Result<(), String> {
    let view = prepare_native_view(file, editor)?;
    open_window(view)
}

#[cfg(feature = "native-view")]
fn open_window(view: NativeView) -> Result<(), String> {
    let title = view.title.clone();
    let root = view.root.clone();
    let html = view.html.clone();
    let shell = view.shell.clone();
    let path = view.path.clone();
    let url: url::Url = native_index_url()
        .parse()
        .map_err(|e| format!("view url: {e}"))?;
    tauri::Builder::default()
        .manage(EditorState { path, root: root.clone() })
        .invoke_handler(tauri::generate_handler![
            preview_html,
            save_deck,
            render_deck,
            move_slide,
            replace_slide
        ])
        .register_uri_scheme_protocol("deck", move |_ctx, request| {
            let range = request
                .headers()
                .get(tauri::http::header::RANGE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let asset = deck_asset(
                &NativeView {
                    title: String::new(),
                    html: html.clone(),
                    source: String::new(),
                    root: root.clone(),
                    path: PathBuf::new(),
                    shell: shell.clone(),
                    editor: false,
                },
                request.uri().path(),
                range.as_deref(),
            );
            let mut response = tauri::http::Response::builder()
                .status(asset.status)
                .header(tauri::http::header::CONTENT_TYPE, asset.content_type);
            if asset.accept_ranges {
                response = response.header(tauri::http::header::ACCEPT_RANGES, "bytes");
            }
            if let Some(content_range) = &asset.content_range {
                response = response.header(tauri::http::header::CONTENT_RANGE, content_range);
            }
            response.body(asset.body).unwrap_or_else(|_| {
                tauri::http::Response::builder()
                    .status(500)
                    .body(Vec::new())
                    .expect("empty response")
            })
        })
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "view", tauri::WebviewUrl::External(url))
                .title(title)
                .inner_size(1440.0, 810.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| e.to_string())
}

#[cfg(feature = "native-view")]
struct EditorState {
    path: PathBuf,
    root: PathBuf,
}

#[cfg(feature = "native-view")]
#[tauri::command]
fn preview_html(markdown: String, state: tauri::State<'_, EditorState>) -> Result<String, String> {
    render_markdown(&markdown, &state.root)
}

#[cfg(feature = "native-view")]
#[tauri::command]
fn save_deck(markdown: String, state: tauri::State<'_, EditorState>) -> Result<(), String> {
    save_markdown(&state.path, &markdown)
}

#[cfg(feature = "native-view")]
#[tauri::command]
fn render_deck(markdown: String, state: tauri::State<'_, EditorState>) -> Result<DeckRender, String> {
    render_model(&markdown, &state.root)
}

#[cfg(feature = "native-view")]
#[tauri::command]
fn move_slide(
    markdown: String,
    from: usize,
    to: usize,
    state: tauri::State<'_, EditorState>,
) -> Result<DeckRender, String> {
    let markdown = reorder_markdown(&markdown, from, to)?;
    render_model(&markdown, &state.root)
}

#[cfg(feature = "native-view")]
#[tauri::command]
fn replace_slide(
    markdown: String,
    index: usize,
    slide: String,
    state: tauri::State<'_, EditorState>,
) -> Result<DeckRender, String> {
    let markdown = replace_slide_markdown(&markdown, index, &slide)?;
    render_model(&markdown, &state.root)
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
        content_range: None,
        accept_ranges: false,
        body: b"not found".to_vec(),
    }
}

fn file_asset(content_type: &'static str, body: Vec<u8>, range: Option<&str>) -> DeckAsset {
    let Some(spec) = range.and_then(|h| h.trim().strip_prefix("bytes=")) else {
        return DeckAsset {
            status: 200,
            content_type,
            content_range: None,
            accept_ranges: true,
            body,
        };
    };
    let total = body.len();
    let Some((start, end)) = parse_byte_range(spec, total) else {
        return DeckAsset {
            status: 416,
            content_type: "text/plain; charset=utf-8",
            content_range: Some(format!("bytes */{total}")),
            accept_ranges: true,
            body: b"range not satisfiable".to_vec(),
        };
    };
    DeckAsset {
        status: 206,
        content_type,
        content_range: Some(format!("bytes {start}-{end}/{total}")),
        accept_ranges: true,
        body: body[start..=end].to_vec(),
    }
}

/// `start-end`, `start-`, or `-suffix` against a body of `total` bytes.
fn parse_byte_range(spec: &str, total: usize) -> Option<(usize, usize)> {
    if total == 0 || spec.contains(',') {
        return None;
    }
    let (start_s, end_s) = spec.split_once('-')?;
    if start_s.is_empty() {
        let suffix: usize = end_s.parse().ok()?;
        if suffix == 0 || suffix > total {
            return None;
        }
        return Some((total - suffix, total - 1));
    }
    let start: usize = start_s.parse().ok()?;
    if start >= total {
        return None;
    }
    let end = if end_s.is_empty() {
        total - 1
    } else {
        let end: usize = end_s.parse().ok()?;
        end.min(total - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
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
        assert!(url.ends_with("/app.html"), "{url}");
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
        let view = prepare_native_view(&path, false).unwrap();
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
        let err = prepare_native_view(&path, false).unwrap_err();
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
        let view = prepare_native_view(&path, true).unwrap();

        let index = deck_asset(&view, "/", None);
        assert_eq!(index.status, 200);
        assert!(index.content_type.starts_with("text/html"));
        assert!(std::str::from_utf8(&index.body).unwrap().contains("Slide"));

        let media = deck_asset(&view, "/images/pic.png", None);
        assert_eq!(media.status, 200);
        assert!(media.accept_ranges);
        assert_eq!(media.content_type, "image/png");
        assert_eq!(media.body, b"PNGDATA");

        let partial = deck_asset(&view, "/images/pic.png", Some("bytes=0-2"));
        assert_eq!(partial.status, 206);
        assert_eq!(partial.body, b"PNG");
        assert_eq!(
            partial.content_range.as_deref(),
            Some("bytes 0-2/7")
        );

        let tail = deck_asset(&view, "/images/pic.png", Some("bytes=4-"));
        assert_eq!(tail.status, 206);
        assert_eq!(tail.body, b"ATA");

        let encoded = deck_asset(&view, "/images/%70ic.png", None);
        assert_eq!(encoded.status, 200);
        assert_eq!(encoded.body, b"PNGDATA");

        assert_eq!(deck_asset(&view, "/../Cargo.toml", None).status, 403);
        assert_eq!(deck_asset(&view, "/%2e%2e/Cargo.toml", None).status, 403);
        assert_eq!(deck_asset(&view, "/images/missing.png", None).status, 404);
        let app = deck_asset(&view, "/app.html", None);
        let page = std::str::from_utf8(&app.body).unwrap();
        assert!(page.contains("data-mode=\"visual\""));
        assert!(page.contains("data-mode=\"overview\""));
        assert!(page.contains("data-mode=\"markdown\""));
        assert!(page.contains("id=\"divider-side\""));
        assert!(page.contains("id=\"format\""));
        assert!(page.contains("mode-visual"));
        assert!(page.contains("# Slide"));
        assert_eq!(
            deck_asset(&view, "/images/pic.png", Some("bytes=99-100")).status,
            416
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_rerender_roundtrip() {
        let dir = std::env::temp_dir().join("keynote-view-save");
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_deck(&dir, "---\ntitle: Old\n---\n\n# Old title\n");
        save_markdown(&path, "---\ntitle: New\n---\n\n# New title\n").unwrap();
        let view = prepare_native_view(&path, true).unwrap();
        assert_eq!(view.path, path);
        assert_eq!(view.title, "New");
        assert!(view.html.contains("New title"), "{}", view.html);
        let html = render_markdown("# Just this\n", &dir).unwrap();
        assert!(html.contains("Just this"), "{html}");
        assert!(render_markdown("", &dir).unwrap_err().contains("no slides"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reorder_keeps_frontmatter_and_moves_the_slide() {
        let md = "---\ntitle: T\n---\n\n# A\n\n---\n\n# B\n\n---\n\n# C\n";
        let out = reorder_markdown(md, 0, 2).unwrap();
        let deck = crate::deck::Deck::from_markdown(&out);
        assert_eq!(deck.frontmatter.title.as_deref(), Some("T"));
        assert_eq!(deck.slides[0].title.as_deref(), Some("B"));
        assert_eq!(deck.slides[1].title.as_deref(), Some("C"));
        assert_eq!(deck.slides[2].title.as_deref(), Some("A"));
        let edited = replace_slide_markdown(&out, 0, "# B2\n\nstill here").unwrap();
        let deck = crate::deck::Deck::from_markdown(&edited);
        assert_eq!(deck.slides[0].title.as_deref(), Some("B2"));
        assert!(replace_slide_markdown(md, 0, "  \n").is_err());
    }

    #[test]
    #[cfg(not(feature = "native-view"))]
    fn open_without_feature_names_the_rebuild() {
        let dir = std::env::temp_dir().join("keynote-view-nofeature");
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_deck(&dir, "# Only\n");
        let err = open(&path, false).unwrap_err();
        assert!(err.contains("native-view"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
