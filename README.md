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
keynote view talk.md
```

Agents: start with `keynote format` (whole slide format, one read) and `keynote skill` / `keynote skill install`.

## Format

Optional frontmatter (`title/author/theme/font`), slides split on exact `---` lines, `![fit|span|loop|muted|autoplay=false|background=blur|auto|theme|#fff](file)` media with basename lookup in `images/`/`videos/`. See `keynote format`.

## Safety net

Successful `new`/`export` writes timestamped `.bak` copies to `.keynote-backups/` beside the deck plus a recovery snapshot under `~/.local/state/keynote/recovery/` (or `$XDG_STATE_HOME`). `history` lists, `restore` rolls back. Unfinished code fences block export; `render` still emits PNGs with an error banner so agents can see failures.

## Desktop app

Decks are Markdown (`.md`). With the native view compiled in:

```sh
cargo build --release --features native-view
./target/release/keynote install
keynote
keynote talk.md
keynote --help
keynote --version
```

`keynote` opens the editor. `keynote talk.md` opens that deck. `keynote install` copies the binary to `~/.local/bin` and writes a desktop launcher (`Exec=keynote %f`), so the shortcut and the CLI are the same program. `~/.local/bin` has to be on `PATH`.

Visual mode edits one slide at a time. Speaker notes (`:::notes`), alignment (`@align`, `@valign`), and backgrounds (`@background color=#112233`, `gradient=#111,#333`, or `image=photo.jpg`) are stored in the Markdown and kept off the slide copy. Presenter mode shows the notes beside the slide. In an exported HTML deck, press `n` to show them.

Images and videos are one per slide (`![fit](file)` or `![span](file)`), copied into `images/` or `videos/`. Fit and span are the positioning controls. A file dropped on the window is attached to the current slide. Freeform x/y boxes, rotation, and crop handles are not part of this Markdown deck.

Undo is Ctrl+Z, redo is Ctrl+Y, duplicate is Ctrl+D. Edits autosave.

## Native view

`keynote view talk.md` opens the deck in a native window, in the Omarchy Tokyo Night chrome. **Visual** edits the current slide (the `---` separators stay in the file). **Overview** is a grid of the slides. **Markdown** is the whole file beside the preview. Drag slides in the list or the grid to reorder them, and drag the dividers to resize the list, the editor, and the preview. Save writes the file (a `.bak` is kept). `--editor` starts in Visual.

```sh
keynote view talk.md --editor
```

Build that binary with WebKit installed:

```sh
cargo build --release --features native-view
```

That feature needs Rust 1.90 or newer, because Tauri does. The rest of the CLI builds without it.

Without the feature, `view` still parses the deck and tells you to rebuild.

## Next milestone

Overview grid on top of this native view. PPTX embeds per-slide PNGs + MP4s (no editable text, matching Hype's rendered-appearance contract).
