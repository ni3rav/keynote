# 03: Check validator text and JSON

**What to build:** `check` reports every deck problem at once with slide and line, gating scripts and exports.

**Blocked by:** 01: Deck format freeze and media layout parsing.

**Status:** ready-for-agent

- [ ] Errors: missing media, invalid layout option, unfinished fence, >1 media per slide; warnings: overfull text, non-16:9 video without fit
- [ ] Text output and `--json` structured diagnostics with slide index + line, exit 0 clean / 1 on errors, errors on stderr
- [ ] Unfinished fences block export; warnings never block
- [ ] Fixture decks cover each diagnostic code
