# keynote

Markdown-powered slides / keynote-style presentation binary (Rust).

## Agent skills

### Issue tracker

Issues live in GitHub Issues via `gh` CLI. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: `GLOSSARY.md` + `docs/adr/` at repo root. See `docs/agents/domain.md`.

## Project

- Binary: `keynote` (`src/main.rs`)
- `cargo run -- <cmd>` / `cargo build --release` → `target/release/keynote`
- Commands: `init`, `parse`, `export`, `present`
- Deck format: Markdown, slides split on a line containing exactly `---`. Optional YAML frontmatter (`title`, `author`, `theme`).
- Skills: `.agents/skills/` (mattpocock/skills via `skills.sh`). Core flow: `/grill-with-docs` → `/to-spec` → `/to-tickets` → `/implement` → `/code-review`.
- `GLOSSARY.md` / `docs/adr/` are created lazily by `/grill-with-docs` + `/domain-modeling`; don't pre-create empty ones.
