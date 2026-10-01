//! Slide chrome that stays in the Markdown but is not slide copy:
//! speaker notes, alignment, and the slide background.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideMeta {
    pub notes: String,
    pub align: String,
    pub valign: String,
    pub background: String,
}

impl Default for SlideMeta {
    fn default() -> Self {
        Self {
            notes: String::new(),
            align: String::new(),
            valign: String::new(),
            background: String::new(),
        }
    }
}

pub fn parse_meta(source: &str) -> SlideMeta {
    let mut meta = SlideMeta::default();
    let mut notes = Vec::new();
    let mut in_notes = false;
    for line in source.lines() {
        let t = line.trim();
        if in_notes {
            if t == ":::" {
                in_notes = false;
            } else {
                notes.push(line.trim_end());
            }
            continue;
        }
        if t == ":::notes" {
            in_notes = true;
            continue;
        }
        if let Some(rest) = t.strip_prefix("@align ") {
            meta.align = normalize_align(rest);
        } else if let Some(rest) = t.strip_prefix("@valign ") {
            meta.valign = normalize_valign(rest);
        } else if let Some(rest) = t.strip_prefix("@background ") {
            meta.background = rest.trim().to_string();
        }
    }
    meta.notes = notes.join("\n").trim().to_string();
    meta
}

/// Markdown that should be drawn on the slide.
pub fn visible_body(source: &str) -> String {
    let mut out = Vec::new();
    let mut in_notes = false;
    for line in source.lines() {
        let t = line.trim();
        if in_notes {
            if t == ":::" {
                in_notes = false;
            }
            continue;
        }
        if t == ":::notes" {
            in_notes = true;
            continue;
        }
        if t.starts_with("@align ") || t.starts_with("@valign ") || t.starts_with("@background ") {
            continue;
        }
        out.push(line);
    }
    out.join("\n").trim().to_string()
}

pub fn with_meta(source: &str, meta: &SlideMeta) -> String {
    let body = visible_body(source);
    let mut lines = Vec::new();
    if matches!(meta.align.as_str(), "center" | "right" | "justify") {
        lines.push(format!("@align {}", meta.align));
    }
    if matches!(meta.valign.as_str(), "middle" | "bottom") {
        lines.push(format!("@valign {}", meta.valign));
    }
    if !meta.background.trim().is_empty() && meta.background.trim() != "none" {
        lines.push(format!("@background {}", meta.background.trim()));
    }
    if !body.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(body.clone());
    }
    if !meta.notes.trim().is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(":::notes".into());
        lines.push(meta.notes.trim().to_string());
        lines.push(":::".into());
    }
    if lines.is_empty() {
        body
    } else {
        lines.join("\n")
    }
}

pub fn set_layout_token(source: &str, layout: &str) -> Result<String, String> {
    let layout = match layout {
        "fit" | "span" => layout,
        _ => return Err("layout must be fit or span".into()),
    };
    let mut found = false;
    let mut out = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if !found && t.starts_with("![") && t.contains("](") && t.ends_with(')') {
            found = true;
            out.push(rewrite_media_layout(t, layout)?);
        } else {
            out.push(line.to_string());
        }
    }
    if !found {
        return Err("this slide has no image or video".into());
    }
    Ok(out.join("\n"))
}

pub fn set_media_line(source: &str, filename: &str, video: bool) -> String {
    let token = if video { "fit" } else { "fit" };
    let line = format!("![{token}]({filename})");
    let mut found = false;
    let mut out = Vec::new();
    for existing in source.lines() {
        let t = existing.trim();
        if !found && t.starts_with("![") && t.contains("](") && t.ends_with(')') {
            found = true;
            out.push(line.clone());
        } else {
            out.push(existing.to_string());
        }
    }
    if !found {
        if !out.is_empty() && !out.last().unwrap().is_empty() {
            out.push(String::new());
        }
        out.push(line);
    }
    out.join("\n")
}

/// CSS class list and a safe inline style for the slide frame.
pub fn frame_attrs(meta: &SlideMeta) -> (String, String) {
    let mut class = String::new();
    if matches!(meta.align.as_str(), "center" | "right" | "justify") {
        class.push_str(" align-");
        class.push_str(&meta.align);
    }
    if matches!(meta.valign.as_str(), "middle" | "bottom") {
        class.push_str(" valign-");
        class.push_str(&meta.valign);
    }
    (class, background_style(&meta.background))
}

fn background_style(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() || raw == "none" {
        return String::new();
    }
    if let Some(color) = raw.strip_prefix("color=") {
        if let Some(c) = safe_hex(color.trim()) {
            return format!("background:{c};");
        }
        return String::new();
    }
    if let Some(spec) = raw.strip_prefix("gradient=") {
        let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
        if parts.len() == 2 {
            if let (Some(a), Some(b)) = (safe_hex(parts[0]), safe_hex(parts[1])) {
                return format!("background:linear-gradient(160deg,{a},{b});");
            }
        }
        return String::new();
    }
    if let Some(spec) = raw.strip_prefix("image=") {
        let mut bits = spec.split_whitespace();
        let src = bits.next().unwrap_or("");
        let fit = bits.next().unwrap_or("span");
        if !safe_path(src) {
            return String::new();
        }
        let size = if fit == "fit" { "contain" } else { "cover" };
        return format!("background-image:url('{src}');background-size:{size};background-position:center;background-repeat:no-repeat;");
    }
    if let Some(c) = safe_hex(raw) {
        return format!("background:{c};");
    }
    String::new()
}

fn rewrite_media_layout(line: &str, layout: &str) -> Result<String, String> {
    let open = line.find('[').ok_or("bad media line")?;
    let close = line.find("](").ok_or("bad media line")?;
    let alt = &line[open + 1..close];
    let mut tokens: Vec<&str> = alt
        .split_whitespace()
        .filter(|t| *t != "fit" && *t != "span")
        .collect();
    tokens.insert(0, layout);
    let new_alt = tokens.join(" ");
    Ok(format!("![{new_alt}]{}", &line[close + 1..]))
}

fn normalize_align(raw: &str) -> String {
    match raw.trim() {
        "left" | "center" | "right" | "justify" => raw.trim().into(),
        _ => String::new(),
    }
}

fn normalize_valign(raw: &str) -> String {
    match raw.trim() {
        "top" | "middle" | "bottom" => raw.trim().into(),
        _ => String::new(),
    }
}

fn safe_hex(raw: &str) -> Option<String> {
    let t = raw.trim();
    if !t.starts_with('#') || !(t.len() == 4 || t.len() == 7 || t.len() == 9) {
        return None;
    }
    if t.chars().skip(1).all(|c| c.is_ascii_hexdigit()) {
        Some(t.to_string())
    } else {
        None
    }
}

fn safe_path(src: &str) -> bool {
    !src.is_empty()
        && !src.contains("..")
        && !src.contains(['"', '\'', '(', ')', ';', '<', '>', '\\'])
        && !src.starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_stay_out_of_the_visible_slide() {
        let src = "# Hello\n\n:::notes\nSay this slowly.\n:::\n";
        let meta = parse_meta(src);
        assert_eq!(meta.notes, "Say this slowly.");
        assert_eq!(visible_body(src), "# Hello");
        let again = with_meta("# Hello", &meta);
        assert_eq!(parse_meta(&again).notes, "Say this slowly.");
        assert!(!visible_body(&again).contains("slowly"));
    }

    #[test]
    fn background_and_align_roundtrip_and_reject_injection() {
        let meta = SlideMeta {
            notes: String::new(),
            align: "center".into(),
            valign: "middle".into(),
            background: "color=#112233".into(),
        };
        let src = with_meta("# Title", &meta);
        let parsed = parse_meta(&src);
        assert_eq!(parsed.align, "center");
        assert_eq!(parsed.valign, "middle");
        let (_class, style) = frame_attrs(&parsed);
        assert!(style.contains("#112233"), "{style}");
        assert!(!style.contains("expression"));
        let (_c, bad) = frame_attrs(&SlideMeta {
            background: "color=red;}</style>".into(),
            ..SlideMeta::default()
        });
        assert!(bad.is_empty(), "{bad}");
    }

    #[test]
    fn layout_token_rewrites_the_first_media_line() {
        let src = "# T\n\n![loop](demo.mp4)\n";
        let out = set_layout_token(src, "span").unwrap();
        assert!(out.contains("![span loop](demo.mp4)"), "{out}");
        assert!(set_layout_token("# T\n", "fit").is_err());
    }
}
