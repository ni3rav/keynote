use crate::deck::{markdown_to_html, resolve_media, Deck, MediaBackground, MediaLayout};
use std::path::{Path, PathBuf};

pub const EXAMPLE_DECK: &str = r#"---
title: My Talk
author: You
theme: dark
font: system-ui
---

# My Talk

Press → to continue

---

## Agenda

- Why markdown slides?
- Demo
- Q&A

---

# Thank you!

Questions?
"#;

pub fn theme_css(name: &str) -> &'static str {
    match name {
        "light" => ":root{--bg:#fafafa;--fg:#1a1a1a;--muted:#666;--code-bg:#eee;--accent:#0b5fff;}",
        "tokyo-night" => ":root{--bg:#1a1b26;--fg:#c0caf5;--muted:#565f89;--code-bg:#24283b;--accent:#7aa2f7;}",
        "paper" => ":root{--bg:#fdf6e3;--fg:#586e75;--muted:#93a1a1;--code-bg:#eee8d5;--accent:#b58900;}",
        _ => ":root{--bg:#111;--fg:#eee;--muted:#888;--code-bg:#222;--accent:#7aa2f7;}",
    }
}

fn deck_theme(deck: &Deck, override_: Option<&str>) -> String {
    if let Some(t) = override_ {
        return t.to_string();
    }
    deck.frontmatter
        .theme
        .clone()
        .unwrap_or_else(|| "dark".to_string())
}

fn deck_font(deck: &Deck) -> String {
    deck.frontmatter
        .font
        .clone()
        .unwrap_or_else(|| "system-ui,sans-serif".to_string())
}

fn relative_media_src(base_dir: &Path, src: &str) -> String {
    if let Some(resolved) = resolve_media(base_dir, src) {
        if let Ok(rel) = resolved.strip_prefix(base_dir) {
            return rel.to_string_lossy().replace('\\', "/");
        }
        return resolved.to_string_lossy().replace('\\', "/");
    }
    src.to_string()
}

fn strip_media_lines(source: &str) -> String {
    crate::slide_meta::visible_body(source)
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("![") && t.contains("](") && t.ends_with(')'))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn media_html(deck_slide: &crate::deck::Slide, base_dir: &Path) -> String {
    let Some(m) = deck_slide.media.first() else {
        return String::new();
    };
    let src = relative_media_src(base_dir, &m.src);
    let layout = match m.layout {
        MediaLayout::Fit => "fit",
        MediaLayout::Span => "span",
    };
    let bg = match &m.background {
        MediaBackground::None => "",
        MediaBackground::Auto => " bg-auto",
        MediaBackground::Blur => " bg-blur",
        MediaBackground::Theme => " bg-theme",
        MediaBackground::Color(c) => {
            // inline style handled below via data attr; keep class
            let _ = c;
            " bg-color"
        }
    };
    let bg_style = match &m.background {
        MediaBackground::Color(c) => format!(" style=\"--media-bg:{c}\""),
        _ => String::new(),
    };
    if m.is_video() {
        let mut attrs = String::from("controls playsinline preload=\"auto\"");
        if m.loop_video {
            attrs.push_str(" loop");
        }
        if m.muted {
            attrs.push_str(" muted");
        }
        if m.autoplay {
            attrs.push_str(" autoplay");
        }
        format!(
            "<div class=\"media-wrap {layout}{bg}\"{bg_style}><video src=\"{src}\" {attrs}></video></div>"
        )
    } else {
        format!("<div class=\"media-wrap {layout}{bg}\"{bg_style}><img src=\"{src}\" alt=\"\" /></div>")
    }
}

fn slide_inner_html(slide: &crate::deck::Slide, base_dir: &Path) -> String {
    let meta = crate::slide_meta::parse_meta(&slide.source);
    let text = strip_media_lines(&slide.source);
    let mut html = markdown_to_html(&text);
    if meta.reveal {
        html = mark_reveal_steps(&html);
    }
    let media = media_html(slide, base_dir);
    let body = if media.is_empty() {
        html
    } else {
        format!("{media}\n<div class=\"overlay\">\n{html}\n</div>")
    };
    let (class, style) = crate::slide_meta::frame_attrs(&meta);
    let notes = html_escape(&meta.notes);
    format!(
        "<div class=\"slide-frame{class}\" style=\"{style}\">\n{body}\n<aside class=\"notes\">{notes}</aside>\n</div>"
    )
}

/// List items become steps. Without a list, paragraphs and later headings do.
fn mark_reveal_steps(html: &str) -> String {
    let tags: &[&str] = if html.contains("<li") {
        &["li"]
    } else {
        &["p", "blockquote", "pre", "h2", "h3"]
    };
    let mut out = String::new();
    let mut rest = html;
    let mut step = 0usize;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let matched = tags.iter().find(|tag| {
            let head = format!("<{tag}");
            rest.starts_with(&head)
                && rest
                    .as_bytes()
                    .get(head.len())
                    .map(|b| *b == b'>' || b.is_ascii_whitespace())
                    .unwrap_or(false)
        });
        if let Some(tag) = matched {
            out.push('<');
            out.push_str(tag);
            out.push_str(&format!(" class=\"step\" data-step=\"{step}\""));
            step += 1;
            rest = &rest[1 + tag.len()..];
        } else {
            out.push('<');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn base_css() -> &'static str {
    r#"
body { margin:0; background:var(--bg); color:var(--fg); font-family:var(--font); }
.slide { max-width:1100px; margin:0 auto; padding:6vh 5vw; font-size:1.5rem; line-height:1.5; min-height:92vh; box-sizing:border-box; position:relative; overflow:hidden; }
.slide h1,h2,h3 { line-height:1.15; }
.slide p { white-space:pre-line; }
.slide pre { background:var(--code-bg); padding:1em; overflow:auto; border-radius:8px; }
.slide code { background:var(--code-bg); padding:.1em .3em; border-radius:4px; }
.slide pre code { background:transparent; padding:0; }
.media-wrap { margin:0 0 1em 0; }
.media-wrap img,.media-wrap video { display:block; max-width:100%; border-radius:8px; }
.media-wrap.fit img,.media-wrap.fit video { max-height:62vh; width:auto; margin:0 auto; object-fit:contain; }
.media-wrap.span img,.media-wrap.span video { width:100%; height:62vh; object-fit:cover; }
.media-wrap.bg-blur { position:relative; }
.media-wrap.bg-auto { background:linear-gradient(135deg,var(--code-bg),var(--bg)); padding:8px; }
.media-wrap.bg-theme { background:var(--bg); }
.media-wrap.bg-color { background:var(--media-bg,#222); padding:8px; }
.slide:has(.media-wrap) .overlay { color:#fff; text-shadow:0 1px 12px rgba(0,0,0,.7); }
.slide:has(.media-wrap) .overlay::before { content:""; position:absolute; inset:0; background:rgba(0,0,0,.28); backdrop-filter:blur(2px); z-index:-1; }
.slide-frame { min-height:70vh; }
.align-center, .align-center .overlay { text-align:center; }
.align-right, .align-right .overlay { text-align:right; }
.align-justify, .align-justify .overlay { text-align:justify; }
.valign-middle { display:flex; flex-direction:column; justify-content:center; }
.valign-bottom { display:flex; flex-direction:column; justify-content:flex-end; }
body.presenting .step { opacity: 0; }
body.presenting .step.on { opacity: 1; transition: opacity .16s ease; }
.notes { display:none; }
body.presenter .notes { display:block; position:fixed; left:0; right:0; bottom:0; max-height:30vh; overflow:auto; margin:0; padding:16px 28px; background:rgba(10,10,16,.88); color:#fff; font-size:1.05rem; white-space:pre-wrap; }
#error-banner { background:#7f1d1d; color:#fff; padding:.6em 1em; border-radius:8px; margin-bottom:1em; font-size:1rem; }
#hud { position:fixed; bottom:12px; right:16px; opacity:.6; font-size:.9rem; }
#hud button { margin-left:8px; }
"#
}

pub fn export_html(
    deck: &Deck,
    base_dir: &Path,
    title_override: Option<String>,
    theme_override: Option<String>,
) -> String {
    let title = title_override
        .or_else(|| deck.frontmatter.title.clone())
        .or_else(|| deck.slides.first().and_then(|s| s.title.clone()))
        .unwrap_or_else(|| "keynote".to_string());
    let theme = deck_theme(deck, theme_override.as_deref());
    let font = deck_font(deck);

    let mut sections = String::new();
    for (i, slide) in deck.slides.iter().enumerate() {
        let inner = slide_inner_html(slide, base_dir);
        sections.push_str(&format!(
            "<section class=\"slide\" data-index=\"{i}\" style=\"display:none\">\n{inner}\n</section>\n"
        ));
    }

    let css = theme_css(&theme);
    let base = base_css();
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
{css}
body {{ --font:{font}; }}
{base}
</style>
</head>
<body>
{sections}
<div id="hud"><span id="pos"></span><button onclick="prev()">←</button><button onclick="next()">→</button></div>
<script>
let i=0, step=-1, presenting=false;
const slides=[...document.querySelectorAll('.slide')];
function stepsOf(s){{ return [...s.querySelectorAll('.step')]; }}
function applySteps(){{ const all=stepsOf(slides[i]); all.forEach((el,n)=>el.classList.toggle('on', !presenting || n<=step)); }}
function show(n){{ i=(n+slides.length)%slides.length; step=-1; slides.forEach((s,k)=>s.style.display=k===i?'block':'none'); document.getElementById('pos').textContent=(i+1)+' / '+slides.length; applySteps(); const v=slides[i].querySelector('video[autoplay]'); if(v){{v.currentTime=0; v.play().catch(()=>{{}});}} try{{ parent.postMessage({{type:'keynote-slide', index:i}}, '*'); }}catch(_ ){{}} }}
function next(){{ const all=stepsOf(slides[i]); if(presenting && step+1<all.length){{ step++; applySteps(); return; }} if(i+1<slides.length) show(i+1); }}
function prev(){{ if(presenting && step>=0){{ step--; applySteps(); return; }} if(i>0) show(i-1); }}
document.addEventListener('keydown',e=>{{ if(e.key==='ArrowRight'||e.key===' '||e.key==='Enter')next(); if(e.key==='ArrowLeft')prev(); if(e.key==='n'||e.key==='N')document.body.classList.toggle('presenter'); }});
window.addEventListener('message',e=>{{ if(!e.data||e.data.type!=='keynote-mode')return; presenting=!!e.data.presenting; document.body.classList.toggle('presenting', presenting); step=-1; applySteps(); }});
show(0);
</script>
</body>
</html>
"#
    )
}

pub fn slide_html(
    deck: &Deck,
    base_dir: &Path,
    index: usize,
    banner: Option<&str>,
    theme_override: Option<String>,
) -> String {
    let theme = deck_theme(deck, theme_override.as_deref());
    let font = deck_font(deck);
    let title = deck.slides.get(index).and_then(|s| s.title.clone()).unwrap_or_default();
    let inner = deck
        .slides
        .get(index)
        .map(|s| slide_inner_html(s, base_dir))
        .unwrap_or_default();
    let banner_html = banner
        .map(|b| format!("<div id=\"error-banner\">{b}</div>"))
        .unwrap_or_default();
    let css = theme_css(&theme);
    let base = base_css();
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=3840">
<title>{title}</title>
<style>
{css}
body {{ --font:{font}; }}
{base}
.slide {{ display:block; }}
</style>
</head>
<body>
<div class="slide">
{banner_html}
{inner}
</div>
</body>
</html>
"#
    )
}

pub fn find_on_path(names: &[&str]) -> Option<PathBuf> {
    let Ok(paths) = std::env::var("PATH") else {
        return None;
    };
    for dir in std::env::split_paths(&paths) {
        for name in names {
            let cand = dir.join(name);
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    None
}

pub fn find_chromium() -> Option<PathBuf> {
    find_on_path(&[
        "chromium-browser",
        "chromium",
        "google-chrome",
        "google-chrome-stable",
    ])
}

pub fn find_ffmpeg() -> Option<PathBuf> {
    find_on_path(&["ffmpeg", "ffmpeg.exe"])
}

/// One program the export path may call.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub present: bool,
    pub path: String,
    pub used_by: String,
}

/// One export format, and whether this machine can write it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FormatStatus {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub reason: String,
}

/// Programs required to export, and which formats that makes possible.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemHealth {
    pub tools: Vec<ToolStatus>,
    pub formats: Vec<FormatStatus>,
}

/// Look up Chromium and ffmpeg. HTML needs neither. PDF and PPTX need Chromium.
/// ffmpeg is required only to embed video that is not already MP4.
pub fn system_health() -> SystemHealth {
    let chromium = find_chromium();
    let ffmpeg = find_ffmpeg();
    let chromium_path = chromium
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let ffmpeg_path = ffmpeg
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let pdf_reason = if chromium.is_some() {
        "Ready".to_string()
    } else {
        "Needs Chromium (chromium or google-chrome) on PATH".to_string()
    };
    let pptx_reason = if chromium.is_none() {
        "Needs Chromium (chromium or google-chrome) on PATH".to_string()
    } else if ffmpeg.is_none() {
        "Ready. Video that is not already MP4 is left out until ffmpeg is installed".to_string()
    } else {
        "Ready".to_string()
    };
    SystemHealth {
        tools: vec![
            ToolStatus {
                id: "chromium".into(),
                name: "Chromium".into(),
                present: chromium.is_some(),
                path: chromium_path,
                used_by: "PDF export, PPTX slide images, and PNG render".into(),
            },
            ToolStatus {
                id: "ffmpeg".into(),
                name: "ffmpeg".into(),
                present: ffmpeg.is_some(),
                path: ffmpeg_path,
                used_by: "PPTX video that is not already MP4, and animated GIF or WebP".into(),
            },
        ],
        formats: vec![
            FormatStatus {
                id: "html".into(),
                label: "HTML".into(),
                available: true,
                reason: "Ready. No extra program.".into(),
            },
            FormatStatus {
                id: "pdf".into(),
                label: "PDF".into(),
                available: chromium.is_some(),
                reason: pdf_reason,
            },
            FormatStatus {
                id: "pptx".into(),
                label: "PPTX".into(),
                available: chromium.is_some(),
                reason: pptx_reason,
            },
        ],
    }
}

pub fn screenshot_png(html_file: &Path, png_file: &Path, width: u32) -> Result<(), String> {
    let chromium = find_chromium().ok_or("no Chromium binary found (tried chromium-browser, chromium, google-chrome)")?;
    let height = width * 9 / 16;
    let url = format!("file://{}", html_file.display());
    let out = std::process::Command::new(chromium)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            &format!("--window-size={width},{height}"),
            &format!("--screenshot={}", png_file.display()),
            &url,
        ])
        .output()
        .map_err(|e| format!("run chromium: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "chromium screenshot failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    if !png_file.exists() {
        return Err("chromium did not write PNG".into());
    }
    Ok(())
}

/// Write `deck` to `output`. `.pdf` prints via Chromium, `.pptx` builds a
/// slide deck, and every other extension is animated HTML.
pub fn write_export(
    deck: &Deck,
    base_dir: &Path,
    output: &Path,
    title: Option<String>,
    theme: Option<String>,
    width: u32,
) -> Result<(), String> {
    let ext = output
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "pdf" {
        let html = export_html(deck, base_dir, title, theme);
        let tmp_html = std::env::temp_dir().join("keynote-export.html");
        let tmp_pdf = std::env::temp_dir().join("keynote-export.pdf");
        std::fs::write(&tmp_html, html).map_err(|e| format!("write tmp: {e}"))?;
        print_pdf(&tmp_html, &tmp_pdf)?;
        std::fs::copy(&tmp_pdf, output).map_err(|e| format!("write {}: {e}", output.display()))?;
    } else if ext == "pptx" {
        let bytes = crate::pptx::export_pptx(deck, base_dir, width)?;
        let tmp = output.with_extension("tmp.pptx");
        std::fs::write(&tmp, &bytes).map_err(|e| format!("write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, output).map_err(|e| format!("write {}: {e}", output.display()))?;
    } else {
        let html = export_html(deck, base_dir, title, theme);
        let tmp = output.with_extension("tmp.html");
        std::fs::write(&tmp, &html).map_err(|e| format!("write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, output).map_err(|e| format!("write {}: {e}", output.display()))?;
    }
    Ok(())
}

pub fn print_pdf(html_file: &Path, pdf_file: &Path) -> Result<(), String> {
    let chromium = find_chromium().ok_or("no Chromium binary found (tried chromium-browser, chromium, google-chrome)")?;
    let url = format!("file://{}", html_file.display());
    let out = std::process::Command::new(chromium)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            "--no-pdf-header-footer",
            &format!("--print-to-pdf={}", pdf_file.display()),
            &url,
        ])
        .output()
        .map_err(|e| format!("run chromium: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "chromium print-to-pdf failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    if !pdf_file.exists() {
        return Err("chromium did not write PDF".into());
    }
    Ok(())
}
