# 02: New scaffold and bundled themes catalog

**What to build:** `new` creates a valid deck folder/file with chosen theme and `themes` lists bundled themes, so humans and agents start from a working file.

**Blocked by:** 01: Deck format freeze and media layout parsing.

**Status:** ready-for-agent

- [ ] `new <path> --title --theme` scaffolds Markdown with frontmatter and example slides + images/videos dirs
- [ ] `themes` (text + `--json`) lists bundled themes (dark/light/tokyo-night/paper) with CSS variables
- [ ] Unknown theme falls back with warning, font falls back to system-ui/monospace
- [ ] Acceptance via CLI: scaffolded file passes `check`
