use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FrontMatter {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub font: Option<String>,
}

pub const BUNDLED_THEMES: &[&str] = &["dark", "light", "tokyo-night", "paper"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MediaLayout {
    Fit,
    Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MediaBackground {
    None,
    Auto,
    Blur,
    Theme,
    Color(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaRef {
    pub alt: String,
    pub src: String,
    pub layout: MediaLayout,
    pub background: MediaBackground,
    pub loop_video: bool,
    pub muted: bool,
    pub autoplay: bool,
    pub line: usize,
    pub invalid_tokens: Vec<String>,
}

impl MediaRef {
    pub fn is_video(&self) -> bool {
        let s = self.src.to_lowercase();
        s.ends_with(".mp4")
            || s.ends_with(".webm")
            || s.ends_with(".mov")
            || s.ends_with(".mkv")
            || s.ends_with(".ogv")
    }
    pub fn is_animated(&self) -> bool {
        let s = self.src.to_lowercase();
        s.ends_with(".gif") || s.ends_with(".webp")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Slide {
    pub index: usize,
    pub source: String,
    pub title: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub media: Vec<MediaRef>,
}

#[derive(Debug, Clone)]
pub struct Deck {
    pub frontmatter: FrontMatter,
    pub slides: Vec<Slide>,
}

impl Deck {
    pub fn from_markdown(input: &str) -> Self {
        let (frontmatter, body, body_start) = split_frontmatter(input);
        let mut slides = Vec::new();
        for (i, (chunk, start, end)) in split_slides(&body, body_start).into_iter().enumerate() {
            let title = first_heading(&chunk);
            let mut media = parse_media_refs(&chunk, start);
            // Default layout: lone media fits, media+heading spans unless explicit.
            for m in media.iter_mut() {
                let explicit = m.alt.contains("fit") || m.alt.contains("span");
                if !explicit {
                    let text_only = chunk
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .filter(|l| !l.trim_start().starts_with("!["))
                        .count();
                    if text_only == 0 {
                        m.layout = MediaLayout::Fit;
                    } else if chunk.lines().any(|l| l.trim_start().starts_with('#')) {
                        m.layout = MediaLayout::Span;
                    } else {
                        m.layout = MediaLayout::Fit;
                    }
                }
                // Hype rule: explicit background switches span -> fit so bg is visible.
                if m.background != MediaBackground::None && m.layout == MediaLayout::Span {
                    m.layout = MediaLayout::Fit;
                }
            }
            slides.push(Slide {
                index: i,
                source: chunk,
                title,
                start_line: start,
                end_line: end,
                media,
            });
        }
        Deck {
            frontmatter,
            slides,
        }
    }

    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        Ok(Self::from_markdown(&content))
    }

    pub fn _titles(&self) -> Vec<String> {
        self.slides
            .iter()
            .map(|s| {
                s.title
                    .clone()
                    .unwrap_or_else(|| format!("Slide {}", s.index + 1))
            })
            .collect()
    }
}

// minimal Result alias to avoid adding anyhow dep for v0.1

fn split_frontmatter(input: &str) -> (FrontMatter, String, usize) {
    // Only treat leading `---\n ... \n---\n` as frontmatter. Returns body + 1-indexed start line.
    let first = input.lines().next();
    if first.map(|l| l.trim() == "---").unwrap_or(false) {
        if let Some((body, start)) = closing_offset(input) {
            let mut parts = input.splitn(3, "---");
            let _empty = parts.next();
            let fm_str = parts.next().unwrap_or("").to_string();
            let fm: FrontMatter = serde_yaml::from_str(&fm_str).unwrap_or_default();
            return (fm, body, start);
        }
    }
    (FrontMatter::default(), input.to_string(), 1)
}

fn closing_offset(input: &str) -> Option<(String, usize)> {
    // Return body after the second `---` line + its 1-indexed start line.
    let mut seen = 0;
    let mut byte_idx = 0;
    let mut line_no = 0usize;
    for line in input.split_inclusive('\n') {
        line_no += 1;
        if line.trim() == "---" {
            seen += 1;
            byte_idx += line.len();
            if seen == 2 {
                return Some((input[byte_idx..].to_string(), line_no + 1));
            }
            continue;
        }
        byte_idx += line.len();
    }
    None
}

fn split_slides(body: &str, body_start: usize) -> Vec<(String, usize, usize)> {
    let mut slides = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut cur_start = body_start;
    let mut line_no = body_start;
    let mut first_in_slide = true;
    for line in body.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        // Keep empty lines inside slides; separator is exact `---`.
        if content.trim() == "---" {
            let text = current.join("\n").trim().to_string();
            if !text.is_empty() {
                slides.push((text, cur_start, line_no - 1));
            }
            current.clear();
            line_no += 1;
            cur_start = line_no;
            first_in_slide = true;
            continue;
        }
        if first_in_slide && content.trim().is_empty() {
            // Skip leading blank lines but advance start.
            cur_start += 1;
        } else {
            first_in_slide = false;
        }
        // Store without trailing newline; re-join later.
        current.push(content.trim_end_matches('\n'));
        line_no += 1;
    }
    let text = current.join("\n").trim().to_string();
    if !text.is_empty() {
        slides.push((text, cur_start, line_no - 1));
    }
    slides
}

fn parse_media_alt(alt: &str, src: &str, line: usize) -> MediaRef {
    let mut layout = MediaLayout::Fit;
    let mut layout_set = false;
    let mut background = MediaBackground::None;
    let mut loop_video = false;
    let mut muted = false;
    let mut autoplay = true;
    let mut invalid = Vec::new();
    for tok in alt.split_whitespace() {
        match tok {
            "fit" => {
                layout = MediaLayout::Fit;
                layout_set = true;
            }
            "span" => {
                layout = MediaLayout::Span;
                layout_set = true;
            }
            "loop" => loop_video = true,
            "muted" => muted = true,
            "autoplay=false" => autoplay = false,
            "autoplay=true" => autoplay = true,
            t if t.starts_with("background=") => {
                let v = t.trim_start_matches("background=");
                match v {
                    "blur" => background = MediaBackground::Blur,
                    "auto" => background = MediaBackground::Auto,
                    "theme" => background = MediaBackground::Theme,
                    "white" => background = MediaBackground::Color("#ffffff".into()),
                    "black" => background = MediaBackground::Color("#000000".into()),
                    _ if v.starts_with('#') => background = MediaBackground::Color(v.into()),
                    _ => invalid.push(tok.into()),
                }
            }
            "" => {}
            _ => invalid.push(tok.into()),
        }
    }
    let _ = layout_set;
    MediaRef {
        alt: alt.into(),
        src: src.into(),
        layout,
        background,
        loop_video,
        muted,
        autoplay,
        line,
        invalid_tokens: invalid,
    }
}

fn parse_media_refs(chunk: &str, chunk_start: usize) -> Vec<MediaRef> {
    // Scan for `![alt](src)` without extra deps.
    let bytes = chunk.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = chunk_start;
    // Map byte offset -> line quickly by precomputing newlines.
    let mut line_at: Vec<usize> = Vec::with_capacity(bytes.len() + 1);
    let mut cur = chunk_start;
    for b in bytes {
        line_at.push(cur);
        if *b == b'\n' {
            cur += 1;
        }
    }
    line_at.push(cur);
    while i + 1 < bytes.len() {
        if bytes[i] == b'!' && bytes[i + 1] == b'[' {
            let alt_start = i + 2;
            if let Some(alt_end) = find_byte(chunk, alt_start, b']') {
                if chunk.as_bytes().get(alt_end + 1) == Some(&b'(') {
                    if let Some(src_end) = find_byte(chunk, alt_end + 2, b')') {
                        let alt = &chunk[alt_start..alt_end];
                        let src = chunk[alt_end + 2..src_end].trim();
                        let l = *line_at.get(i).unwrap_or(&line);
                        let mut m = parse_media_alt(alt, src, l);
                        // Remember explicitness via alt tokens for default rule.
                        if !(alt.contains("fit") || alt.contains("span")) {
                            // Mark as non-explicit by forcing Fit here; caller applies default.
                            // Keep parsed Fit, caller overrides when needed.
                        }
                        // Fix explicit flag: caller checks alt string directly.
                        let _ = &mut m;
                        out.push(m);
                        i = src_end + 1;
                        continue;
                    }
                }
            }
        }
        if bytes[i] == b'\n' {
            line += 1;
        }
        i += 1;
    }
    out
}

fn find_byte(s: &str, from: usize, b: u8) -> Option<usize> {
    s.as_bytes()
        .iter()
        .skip(from)
        .position(|c| *c == b)
        .map(|p| p + from)
}

pub fn resolve_media(base_dir: &std::path::Path, src: &str) -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(src);
    // Direct relative first.
    let direct = base_dir.join(p);
    if direct.exists() {
        return Some(direct);
    }
    // Basename fallback into images/, videos/, same dir.
    if let Some(name) = p.file_name() {
        for dir in ["images", "videos", ""] {
            let cand = if dir.is_empty() {
                base_dir.join(name)
            } else {
                base_dir.join(dir).join(name)
            };
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub slide: usize,
    pub line: usize,
    pub code: String,
    pub severity: String,
    pub message: String,
}

pub fn check_deck(deck: &Deck, base_dir: &std::path::Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    if let Some(t) = deck.frontmatter.theme.as_deref() {
        if !BUNDLED_THEMES.contains(&t) {
            diags.push(Diagnostic {
                slide: 0,
                line: 1,
                code: "W003".into(),
                severity: "warning".into(),
                message: format!("unknown theme '{t}', falling back to dark"),
            });
        }
    }
    for s in &deck.slides {
        let n = s.index + 1;
        if s.media.len() > 1 {
            diags.push(Diagnostic {
                slide: n,
                line: s.media[1].line,
                code: "E004".into(),
                severity: "error".into(),
                message: "slide holds more than one image/video; only the first is used".into(),
            });
        }
        for m in &s.media {
            if !m.invalid_tokens.is_empty() {
                diags.push(Diagnostic {
                    slide: n,
                    line: m.line,
                    code: "E002".into(),
                    severity: "error".into(),
                    message: format!("invalid layout option(s): {}", m.invalid_tokens.join(", ")),
                });
            }
            if m.src.is_empty() {
                diags.push(Diagnostic {
                    slide: n,
                    line: m.line,
                    code: "E001".into(),
                    severity: "error".into(),
                    message: "empty media source".into(),
                });
            } else if resolve_media(base_dir, &m.src).is_none() {
                diags.push(Diagnostic {
                    slide: n,
                    line: m.line,
                    code: "E001".into(),
                    severity: "error".into(),
                    message: format!("missing media '{}'", m.src),
                });
            }
            if m.is_video() && m.layout == MediaLayout::Span {
                diags.push(Diagnostic {
                    slide: n,
                    line: m.line,
                    code: "W002".into(),
                    severity: "warning".into(),
                    message: "spanning video crops playback; prefer fit for non-16:9".into(),
                });
            }
        }
        // Unfinished fence.
        let fences = s.source.lines().filter(|l| l.trim_start().starts_with("```")).count();
        if fences % 2 == 1 {
            diags.push(Diagnostic {
                slide: n,
                line: s.end_line,
                code: "E003".into(),
                severity: "error".into(),
                message: "unfinished code fence".into(),
            });
        }
        let lines = s.source.lines().count();
        if lines > 25 || s.source.len() > 2000 {
            diags.push(Diagnostic {
                slide: n,
                line: s.start_line,
                code: "W001".into(),
                severity: "warning".into(),
                message: "slide holds so much text it may shrink below readable size".into(),
            });
        }
    }
    diags
}

#[derive(Debug, Clone, Serialize)]
pub struct SlideOutline {
    pub number: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub headline: Option<String>,
    pub media: Option<String>,
    pub layout: Option<String>,
}

pub fn outline(deck: &Deck) -> Vec<SlideOutline> {
    deck.slides
        .iter()
        .map(|s| SlideOutline {
            number: s.index + 1,
            start_line: s.start_line,
            end_line: s.end_line,
            headline: s.title.clone(),
            media: s.media.first().map(|m| m.src.clone()),
            layout: s.media.first().map(|m| match m.layout {
                MediaLayout::Fit => "fit".into(),
                MediaLayout::Span => "span".into(),
            }),
        })
        .collect()
}

fn first_heading(chunk: &str) -> Option<String> {
    for line in chunk.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            let title = t.trim_start_matches('#').trim().to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }
    None
}

pub fn markdown_to_html(md: &str) -> String {
    let prepped = underline_quirk(md);
    let parser = pulldown_cmark::Parser::new_ext(
        &prepped,
        pulldown_cmark::Options::all(),
    );
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, parser);
    out
}

/// Hype quirk: `*a*` is italic (native Markdown), `_u_` is underline.
fn underline_quirk(md: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        out.push_str(&underline_line(line));
        out.push('\n');
    }
    out
}

fn underline_line(line: &str) -> String {
    // Replace `_text_` with `<u>text</u>` outside inline code spans.
    let mut res = String::new();
    let mut i = 0;
    let b = line.as_bytes();
    let mut in_code = false;
    while i < b.len() {
        if b[i] == b'`' {
            in_code = !in_code;
            res.push('`');
            i += 1;
            continue;
        }
        if !in_code && b[i] == b'_' {
            if let Some(end) = line[i + 1..].find('_') {
                let inner = &line[i + 1..i + 1 + end];
                if !inner.is_empty() && !inner.contains(char::is_whitespace) {
                    res.push_str("<u>");
                    res.push_str(inner);
                    res.push_str("</u>");
                    i += end + 2;
                    continue;
                }
            }
        }
        res.push(b[i] as char);
        i += 1;
    }
    res
}

pub fn markdown_to_text(md: &str) -> String {
    use pulldown_cmark::{Event, Tag, TagEnd};
    let parser = pulldown_cmark::Parser::new_ext(md, pulldown_cmark::Options::all());
    let mut out = String::new();
    let mut in_heading = false;
    for ev in parser {
        match ev {
            Event::Start(Tag::Heading { .. }) => in_heading = true,
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                out.push('\n');
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                out.push_str(&format!("\n[media: {dest_url}]\n"));
            }
            Event::End(TagEnd::Image) => {}
            Event::Text(t) => {
                if in_heading {
                    out.push_str(&t.to_uppercase());
                } else {
                    out.push_str(&t);
                }
            }
            Event::Code(c) => {
                out.push('`');
                out.push_str(&c);
                out.push('`');
            }
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => out.push('\n'),
            Event::Start(Tag::List(_)) => out.push('\n'),
            Event::Start(Tag::Item) => out.push_str("  • "),
            Event::End(TagEnd::Item) => out.push('\n'),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_exact_dash_line() {
        let md = "# A\nhello\n---\n# B\nworld\n";
        let deck = Deck::from_markdown(md);
        assert_eq!(deck.slides.len(), 2);
        assert_eq!(deck.slides[0].title.as_deref(), Some("A"));
        assert_eq!(deck.slides[1].title.as_deref(), Some("B"));
    }

    #[test]
    fn ignores_dash_inside_text() {
        let md = "# A\n--- not a separator\n---\n# B\n";
        let deck = Deck::from_markdown(md);
        assert_eq!(deck.slides.len(), 2);
    }

    #[test]
    fn parses_frontmatter() {
        let md = "---\ntitle: Hello\nauthor: Ada\n---\n# A\n---\n# B\n";
        let deck = Deck::from_markdown(md);
        assert_eq!(deck.frontmatter.title.as_deref(), Some("Hello"));
        assert_eq!(deck.slides.len(), 2);
    }

    #[test]
    fn empty_deck() {
        let deck = Deck::from_markdown("");
        assert_eq!(deck.slides.len(), 0);
    }

    #[test]
    fn parses_media_layout() {
        let md = "# T\n\n![fit background=blur](pic.jpg)\n";
        let deck = Deck::from_markdown(md);
        assert_eq!(deck.slides.len(), 1);
        let m = &deck.slides[0].media[0];
        assert_eq!(m.src, "pic.jpg");
        assert!(matches!(m.layout, MediaLayout::Fit));
        assert!(matches!(m.background, MediaBackground::Blur));
    }

    #[test]
    fn flags_invalid_layout() {
        let md = "# T\n\n![sideways](pic.jpg)\n";
        let deck = Deck::from_markdown(md);
        assert_eq!(deck.slides[0].media[0].invalid_tokens, vec!["sideways"]);
    }

    #[test]
    fn underline_quirk_works() {
        let html = markdown_to_html("Hello _world_ and *ital*");
        assert!(html.contains("<u>world</u>"), "{html}");
        assert!(html.contains("<em>ital</em>"), "{html}");
    }

    #[test]
    fn detects_unfinished_fence() {
        let md = "# T\n```rust\nlet x = 1;\n";
        let deck = Deck::from_markdown(md);
        let dir = std::path::Path::new(".");
        let diags = check_deck(&deck, dir);
        assert!(diags.iter().any(|d| d.code == "E003"));
    }

    #[test]
    fn detects_missing_media() {
        let md = "# T\n\n![](nope.png)\n";
        let deck = Deck::from_markdown(md);
        let dir = std::path::Path::new(".");
        let diags = check_deck(&deck, dir);
        assert!(diags.iter().any(|d| d.code == "E001"));
    }
}
