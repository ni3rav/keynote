# keynote

Markdown-powered slides in your terminal — a portable, Rust-first Hype-alternative.

Write decks in Markdown, validate and preview headlessly (agent-friendly), present in the terminal, and share as animated HTML, static PDF, or PPTX with embedded video.

## Install (binary)

Download `keynote` from the latest GitHub release, then:

```sh
chmod +x keynote
./keynote --help
```

Requires headless Chromium (`chromium-browser`/`chromium`/`google-chrome`) for `render` and PDF/PPTX export, and `ffmpeg` for video/animated conversion. Everything else is a single static binary.

## Quickstart

```sh
keynote new talk.md --title "My talk" --theme tokyo-night
keynote check talk.md
keynote slides talk.md
keynote render talk.md -o slides/ --width 1920
keynote export talk.md talk.html
keynote export talk.md talk.pdf
keynote export talk.md talk.pptx
keynote present talk.md
```

Agents: start with `keynote format` (whole slide format, one read) and `keynote skill` / `keynote skill install`.

## Format

Optional frontmatter (`title/author/theme/font`), slides split on exact `---` lines, `![fit|span|loop|muted|autoplay=false|background=blur|auto|theme|#fff](file)` media with basename lookup in `images/`/`videos/`. See `keynote format`.

## Safety net

Successful `new`/`export` writes timestamped `.bak` copies to `.keynote-backups/` beside the deck plus a recovery snapshot under `~/.local/state/keynote/recovery/` (or `$XDG_STATE_HOME`). `history` lists, `restore` rolls back. Unfinished code fences block export; `render` still emits PNGs with an error banner so agents can see failures.

## Next milestone

Tauri Visual/Overview/Markdown editor reusing the HTML single renderer (this CLI is the tracer). PPTX embeds per-slide PNGs + MP4s (no editable text, matching Hype's rendered-appearance contract).
