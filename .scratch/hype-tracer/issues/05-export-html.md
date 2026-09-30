# 05: Animated HTML export single renderer

**What to build:** `export` to animated HTML is the single renderer for editor, PNG, and PDF: themed slides, overlaid text-on-image, embedded video with playback flags.

**Blocked by:** 01: Deck format freeze and media layout parsing, 02: New scaffold and bundled themes catalog.

**Status:** ready-for-agent

- [ ] Themed HTML with CSS variables, big headlines, code blocks, visible line breaks, `*` italic / `_` underline quirk
- [ ] Image fit/span + background blur/auto/color, text overlay readable; video fit with loop/muted/autoplay preserved, holds last frame
- [ ] Media resolved relatively beside Markdown; export snapshots input and never clobbers output on failure
- [ ] `export` blocks on `check` errors, warns on warnings
