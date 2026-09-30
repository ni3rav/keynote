## Problem Statement

`keynote` today is a minimal terminal slide viewer with HTML export, while the reference (Hype) shows what users and agents expect: Markdown decks with images/video, layouts, themes, validation, headless render, and animated HTML plus static PDF sharing. Users cannot author media-rich decks, agents cannot validate or preview slides headlessly, and sharing is limited to a single dark static HTML.

## Solution

Bring `keynote` to Hype-parity in Rust, starting with a CLI tracer milestone: a portable, terminal-first Hype-alternative where HTML is the single renderer. New decks scaffold with theme, `check` validates everything at once, `slides` gives an agent-readable outline, `render` produces 4K PNGs plus outline JSON via headless Chromium, and `export` produces animated HTML (embedded video) and static PDF (poster frames). Later milestones add Tauri Visual/Overview/Markdown modes, persistence safety net, and PPTX.

## User Stories

1. As a presenter, I want to scaffold a new deck with a title and theme, so that I start from a valid file.
2. As a presenter, I want to split slides on a `---` line with blank lines around, so that my Markdown stays readable.
3. As a presenter, I want frontmatter `title/author/theme/font` to control look, so that sharing keeps typography.
4. As a presenter, I want `*asterisks*` as italic and `_underscores_` as underline, so that decks copied from Hype render the same.
5. As a presenter, I want big headlines, lists, quotes, tables, inline code and fenced code with language highlighting, so that technical talks work.
6. As a presenter, I want visible line breaks preserved on slides, so that what I type is what I see.
7. As a presenter, I want to add an image with `![](name)` resolved beside the Markdown in `images/`/`videos/` by basename, so that folders stay portable.
8. As an agent, I want basename resolution to find media without exact relative paths, so that generated decks don't break on paths.
9. As a presenter, I want one media per slide with text overlaid (white lettering, darkening, light blur), so that image slides stay readable.
10. As a presenter, I want `![fit]` to show the whole image and `![span]` to fill/crop, so that I control framing.
11. As a presenter, I want `![fit background=blur]` blurred-fill and `![fit background=auto]` edge-matched and `![fit background=#ffffff]` explicit color, so that non-16:9 media looks intentional.
12. As a presenter, I want `White/Black/Use theme color` equivalents via background options, so that I can override auto matching.
13. As a presenter, I want `![loop muted](demo.mp4)` and `![autoplay=false](demo.mp4)` video controls, so that playback intent travels with the deck.
14. As a presenter, I want videos to fit and play once on arrival, holding last frame with Space to replay, so that presenting is predictable.
15. As a presenter, I want animated WebP/GIF to play inline while presenting and export as first-frame in PDF, so that fun loops don't break sharing.
16. As an agent, I want `check` to report every problem at once with slide and line (missing media, invalid layout, unfinished fences, overfull text), so that I can fix in one pass.
17. As an agent, I want `check --json` structured output and exit 0/1 with errors on stderr, so that scripts can gate on it.
18. As an agent, I want `slides` outline (number, lines, headline, media) as text and JSON, so that I can navigate large decks.
19. As an agent, I want `render --slide N -o slide.png` to always write a PNG even for broken slides with a banner, so that I can see what went wrong.
20. As an agent, I want `render -o slides/` to emit every slide PNG at 4K plus `slides.json`, with `--width` override, so that bulk preview works.
21. As a presenter, I want `export deck.md talk.html` animated HTML with embedded video, so that sharing needs no fonts or setup.
22. As a presenter, I want `export deck.md talk.pdf` static PDF with vectors for text and 4K JPEG for images (unused pixels outside span crops omitted), so that files stay small and sharp.
23. As a presenter, I want export to run from the on-disk snapshot at start and leave existing output intact on failure/cancel, so that I can keep editing safely.
24. As a presenter, I want bundled themes (dark/light + two more) and font choice saved in Markdown, so that decks look the same elsewhere when fonts are installed.
25. As a terminal presenter, I want `present` with arrows/Space/Escape, Home/End jump, position header, so that I can present without a GUI.
26. As both users, I want autosave + `.keynote-backups/` + recovery snapshots + version history restore (full vision, tracer prepares paths), so that crashes don't lose work.

## Implementation Decisions

- CLI tracer surface: `new --title --theme`, `check [--json]`, `slides [--json]`, `render (--slide N | all) -o --width [--json]`, `export <md> <html|pdf>`, `themes`. Defer `open`, `skill install`, `help format` to editor milestone.
- Single renderer: HTML export is the source of truth for editor, present preview, PNG screenshot, and PDF print. Terminal `present` renders text fallback from the same Markdown.
- Deck parsing seam: extend existing split-on-exact-`---` plus leading frontmatter seam to enforce blank-line-around rule, basename media resolution, single Heading title extraction, layout-option parsing inside image alt/text.
- Media model: MediaRef with kind (image/video/animated), basename lookup in sibling `images/`/`videos/` plus same-dir fallback, layout enum (Fit/Span), background enum (Auto/Blur/Color/Theme), video flags (loop/muted/autoplay). One media per slide; second media is a `check` error.
- Validator seam: new checker that returns structured diagnostics (slide index, line number, code, message, severity error/warning). Errors: missing media, invalid layout option, unfinished fence, >1 media, bad frontmatter. Warnings: overfull text (shrink below readable), non-16:9 video without fit.
- Outline seam: reuse title + media extraction for `slides` text/JSON (number, start/end lines, headline, media path).
- Render seam: write per-slide standalone HTML from the single renderer, screenshot via headless Chromium at 3840 width (configurable via `--width`), collect `slides.json`. Broken slides render with an error banner but still produce PNG. Errors on stderr, exit codes preserved.
- Export seam: HTML path embeds media (local files referenced relatively, videos with loop/muted/autoplay preserved); PDF path prints the same HTML via headless Chromium `print-to-pdf`, videos represented by poster/first frame. Snapshot input at start; never clobber output on failure.
- Themes seam: bundled theme catalog (dark/light/tokyo-night/paper) with CSS variables for text/code/background; frontmatter `theme/font` selects, CLI `--theme` overrides; fonts fall back to system-ui/monospace. No Omarchy dependency in tracer; opportunistic Omarchy dir read deferred.
- Hype quirks frozen for tracer: `*` = italic, `_` = underline; lone image fits without crop; text-on-image overlaid with darkening + light blur; finished videos hold last frame.
- Highest-seam testing preference: test through Deck parsing, checker diagnostics, outline JSON, and HTML export strings; only shell out to Chromium/FFmpeg in integration tests gated on binaries existing.

## Testing Decisions

- Good tests assert external behavior (CLI stdout/JSON/exit codes, PNG/PDF/HTML artifacts exist, diagnostics contain slide+line), not internal HTML string internals.
- Units: deck split/frontmatter/title, media layout parsing, validator diagnostics, outline numbering.
- Integration: `new` scaffolds valid file; `check` on good/bad fixtures; `slides --json` shape; `render --slide` writes PNG (skipped if no Chromium); `export html/pdf` writes files (PDF skipped if no Chromium).
- Prior art: existing deck split tests in the parsing module; extend the same style for layout parsing and diagnostics.

## Out of Scope

- Tauri Visual/Overview/Markdown editor, sidebar drag/reorder, formatting bar, divider drag.
- PPTX export (embedded MP4, 4K JPEG slides), animated WebP/GIF → MP4 conversion on export.
- Autosave loop, `.keynote-backups/`, recovery snapshots, version history UI (paths reserved but not implemented in tracer).
- Omarchy theme/font discovery, desktop theme following, launcher entries.
- `open`, `skill install`, `help format` agent skill, live file-watch into open editor.
- Terminal inline video playback; terminal holds text + media placeholder only.
- Background blur/match-edge pixel computation beyond CSS (true edge sampling deferred; tracer uses CSS approximation).

## Further Notes

- Full vision converges on Tauri + HTML webview reusing the tracer renderer, so tracer HTML/CSS choices must survive intact into the editor.
- PDF with playable embedded video is explicitly not pursued; PDF is a static snapshot by design, matching Hype.
- Prototype decision encoded here: 4K (3840×2160) screenshot + `slides.json` sidecar as the agent preview contract; trimmed to that shape so later milestones can rely on it.
- Local tracker: no git remote configured, so spec and tickets live under `.scratch/hype-tracer/` until a remote exists.
