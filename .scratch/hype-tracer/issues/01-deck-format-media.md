# 01: Deck format freeze and media layout parsing

**What to build:** the end-to-end parsing behaviour every later ticket relies on: slides split on `---`, frontmatter `title/author/theme/font`, first-heading titles, and Hype-style media refs with layout/background/video flags resolved by basename.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Slides split on exact `---` line, blank-line-around enforced for new decks, legacy decks still parse
- [ ] Frontmatter parses `title/author/theme/font`, titles extracted from first heading
- [ ] Media refs parse `![fit|span|loop|muted|autoplay=false|background=blur|auto|#fff](file)` with basename lookup in `images/`/`videos/`/same-dir
- [ ] One media per slide recorded with kind, layout, background, video flags; unit tests cover parsing
