pub const FORMAT_DOC: &str = r#"keynote slide format (agent-readable, Hype-compatible subset)
================================================================

File: Markdown with optional YAML frontmatter, then slides.

Frontmatter (optional, leading lines):
---
title: My talk
author: You
theme: dark        # dark|light|tokyo-night|paper
font: system-ui
---

Slides: split on a line containing exactly `---` with blank lines around.
The single-slide editor hides those separators; the stored file keeps them.

# Slide one

Text, lists, quotes, tables, inline `code`, fenced code with language:

```rust
fn next() {}
```

Emphasis quirk (Hype-compatible): `*asterisks*` = italic, `_underscores_` = underline.
Ordinary line breaks stay visible (pre-line). Headlines render big.

Media: one image/video per slide, basename resolved in `images/`/`videos/`/same-dir:

![](diagram.png)

Layout inside brackets (space-separated):
  fit | span                              framing (lone image defaults fit, headline+image defaults span)
  loop muted autoplay=false               video flags (default: play once on arrival, hold last frame)
  background=blur | background=auto | background=theme | background=#ffffff | background=white | background=black

Examples:
  ![fit](photo.jpg)                       whole image, text overlaid
  ![span](photo.jpg)                      fill slide, crop as needed
  ![fit background=blur](portrait.jpg)    fitted image over blurred fill
  ![fit background=auto](clip.mp4)        fitted video over edge-matched color
  ![loop muted](demo.mp4)                 looping silent video
  ![autoplay=false](demo.mp4)             wait for Space/click

Text on media slides is overlaid (white, darkening + light blur). Videos fit;
prefer fit for non-16:9. Animated WebP/GIF play inline; PDF uses first frame,
PPTX converts to MP4 when ffmpeg is present.

CLI (exit 0 ok / 1 fail, errors on stderr, --json everywhere):
  keynote new talk.md --title "T" --theme dark
  keynote check talk.md                   # all problems with slide+line
  keynote slides talk.md                  # outline: number, lines, headline, media
  keynote render talk.md --slide 1 -o s.png [--width 3840]
  keynote render talk.md -o slides/       # every PNG + slides.json
  keynote export talk.md talk.html        # animated, embedded video
  keynote export talk.md talk.pdf         # static snapshot via Chromium
  keynote export talk.md talk.pptx        # slides as 4K images + embedded MP4
  keynote themes
  keynote history talk.md / restore talk.md --list / --restore <bak>
  keynote format                          # this document
  keynote skill                           # print agent skill
  keynote skill install                   # install to ~/.agents/skills/keynote/
  keynote open talk.md                    # export HTML, open browser, present in terminal
  keynote present talk.md                 # terminal
  keynote view talk.md                    # native window (cargo build --features native-view)
  keynote view talk.md --editor           # markdown beside the same preview
"#;

pub const SKILL_TEXT: &str = r#"---
name: keynote
description: Build Markdown slide decks with the keynote CLI.
---

# Keynote skill

Decks are Markdown, slides split on exact `---` lines. Read the format once:

`keynote format`

Authoring loop for agents:
1. `keynote new talk.md --title "T" --theme tokyo-night`
2. Write Markdown + media in `images/`/`videos/`, reference by basename.
3. `keynote check talk.md --json` until zero errors.
4. `keynote render talk.md -o slides/ --width 1920` and inspect PNGs + `slides.json`.
5. `keynote export talk.md talk.html` (animated) and `talk.pdf`/`talk.pptx` (static) to share.
"#;

pub fn install_skill() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "no HOME set".to_string())?;
    let dir = std::path::Path::new(&home).join(".agents/skills/keynote");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let dest = dir.join("SKILL.md");
    std::fs::write(&dest, SKILL_TEXT).map_err(|e| format!("write {}: {e}", dest.display()))?;
    Ok(dest)
}
