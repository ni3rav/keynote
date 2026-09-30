# 07: Static PDF export via headless print

**What to build:** `export` to PDF prints the same HTML statically (vectors for text, 4K JPEG for images, poster frames for video), completing the share story.

**Blocked by:** 05: Animated HTML export single renderer.

**Status:** ready-for-agent

- [ ] `export deck.md talk.pdf` via headless Chromium print-to-pdf from snapshot HTML
- [ ] Text stays vector, images sized for visible 4K area, videos show poster/first frame
- [ ] Failure leaves existing PDF intact with stderr + exit 1; missing Chromium errors clearly
- [ ] Integration test gated on Chromium binary presence
