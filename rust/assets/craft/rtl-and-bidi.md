# RTL & Bidi

> Every screen Plan must be mirror-safe unless the target locales are explicitly LTR-only.

## Layout primitives
- Use Figma auto-layout with `direction: HORIZONTAL` and let `itemReverseZIndex` handle mirroring.
- Avoid absolute positioning for anything that participates in reading order.
- Padding uses logical names in prop (`start`, `end`) instead of (`left`, `right`). In Figma we still use left/right under the hood, but component props expose start/end.

## Text
- Text alignment: never hardcode `LEFT` for body — use `START` (maps to LEFT in LTR, RIGHT in RTL).
- Numbers and Latin substrings inside Arabic/Hebrew runs: Figma handles bidi by default; do not override unless you also set base direction.
- Prefer logical icons (magnifier, arrow-start, arrow-end). Keep media-control icons (play, FF, rewind) LTR because the universal convention is LTR.

## Icons that flip
Flip: back arrow, forward arrow, speech bubble tails, progress direction, reading-order indicators.
Do NOT flip: time-dependent icons (clock), media playback, logos, number glyphs, checkmarks, search.

## Mirroring visuals
- Shadows: keep direction (from above). Do not mirror a drop shadow just because layout flipped.
- Charts: data axis retains semantic direction (up is up, forward time is forward time). Only UI chrome around the chart mirrors.

## Figma representation
- Component variants: if a component has a directional visual (e.g., "card with leading avatar + trailing action"), add `dir: ltr | rtl` variant.
- Plan can reference `componentRef` with suffix `@rtl` to force mirrored variant.

## Hard fails (Plan lint)
- text node with `align: LEFT` on a screen whose `targetLocales` includes an RTL locale → RTL-001.
- icon-only button whose icon is in the "must flip" list and component has no RTL variant → RTL-002.
- hardcoded `paddingLeft` / `paddingRight` (not via token `space.*.start`/`end`) on a screen with RTL locales → RTL-003.
