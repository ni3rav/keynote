# 06: Headless render PNG plus slides.json

**What to build:** `render` screenshots the HTML renderer at 4K via headless Chromium, always emitting PNG even for broken slides with a banner.

**Blocked by:** 05: Animated HTML export single renderer.

**Status:** ready-for-agent

- [ ] `render --slide N -o slide.png [--width]` writes PNG with error banner on broken slides
- [ ] `render -o dir/` writes every slide PNG plus `slides.json` outline sidecar
- [ ] Missing Chromium produces clear stderr + exit 1, no partial files left as success
- [ ] Integration test gated on Chromium binary presence
