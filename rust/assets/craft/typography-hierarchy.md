# Typography Hierarchy

> Typography is the primary load-bearer of hierarchy. Use scale and weight before you use color or size-on-nothing.

## Scale
The default DS ships:
- `display` — hero only; ≤ 1 per screen
- `title.lg` — section header; ≤ 3 per screen
- `title.md` — subsection
- `title.sm` — card title
- `body.lg` — lede / intro paragraph
- `body.md` — default body
- `body.sm` — secondary / helper
- `label.md` — form labels, nav items
- `caption.sm` — timestamps, help text, legal

## Use the named role
Every text node MUST reference `typo.*` by its semantic name. Never inline `fontSize: 16`. Bind to the DS; let the DS own the number.

## Rhythm
- Line height follows the scale: body 1.5, titles 1.1–1.3, display 1.05–1.1.
- Don't mix fonts within a screen unless you've declared a `displayFont` + `textFont` pair at the DS level.
- Paragraph spacing = the line height unit (not an arbitrary 16px). Keep the vertical rhythm grid.

## Weight
- Prefer weight contrast over color contrast for hierarchy inside dense information.
- Do not go from `300` (light) to `700` (bold) with nothing in between; that reads as two products.

## Alignment
- Body text: left-aligned (or `start` under RTL). Centered body paragraphs > 2 lines are almost always wrong.
- Short labels and headers can center; tabular data never centers except for single-glyph cells.

## Don'ts (hard fails)
- text node without `styleRef` → TYPO-001.
- more than 2 display fonts on a single screen → TYPO-002.
- body font size < 13 on desktop, < 14 on mobile → TYPO-003.
- centered paragraph > 2 lines → TYPO-004.
