# keynote

Markdown slides. Check and export them from the terminal, or edit and present them in a desktop window.

## Install locally

You need Rust 1.90 or newer (`rustup` is fine). On Linux, the desktop window also needs WebKit:

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Build, install, and open:

```sh
cargo build --release --features native-view
./target/release/keynote install
keynote
```

`keynote install` copies the binary to `~/.local/bin/keynote` and adds a desktop launcher. If `keynote` is not found, add this to your shell profile:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Then:

```sh
keynote talk.md          # open a deck in the editor
keynote --help
keynote --version
```

`render`, PDF, and PPTX also need headless Chromium (`chromium` or `google-chrome`). PPTX video needs `ffmpeg`.

A terminal-only build, without the window:

```sh
cargo build --release
./target/release/keynote --help
```

## Use

```sh
keynote new talk.md --title "My talk" --theme tokyo-night
keynote check talk.md
keynote slides talk.md
keynote present talk.md
keynote export talk.md talk.html
keynote export talk.md talk.pdf
keynote export talk.md talk.pptx
```

Help inside the editor is F1. Agents can run `keynote format` and `keynote skill`.

## Editor

The window has Visual, Overview, Markdown, Presenter, and Present. Drag slides to reorder them. Drag the dividers to resize panes. Edits autosave. Ctrl+Z undoes, Ctrl+Y redoes, Ctrl+D duplicates.

`@reveal` shows bullets one at a time in Present (Space or right arrow). Speaker notes go in `:::notes` and stay off the slide. Alignment and backgrounds are `@align`, `@valign`, and `@background`.

## Format

Optional frontmatter (`title`, `author`, `theme`, `font`). Slides split on a line that is exactly `---`. One image or video per slide, looked up by basename in `images/` or `videos/`:

```md
![fit](photo.jpg)
![span loop muted](demo.mp4)
```

`keynote format` prints the full reference.

## Backups

`new` and `export` keep a `.bak` in `.keynote-backups/` next to the deck, and a recovery copy under `~/.local/state/keynote/recovery/`. `keynote history talk.md` lists them. `keynote restore talk.md` puts one back.
