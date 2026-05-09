# Typography Hierarchy — Editorial

> Rules for editorial / long-read / marketing surfaces. Denser information and lower ergonomic pressure — we can afford more expression.

## Measure
- Body measure 60–80 characters per line. Enforce via max-width on the text column, not via column gaps.
- At < 60 cpl: users fatigue on scan. At > 90 cpl: they lose row.

## Scale (editorial)
- Hero `display`: up to 2× the body scale, e.g. 56–72px on desktop.
- Pull quote: body size + 4–6px bump, italic or serif shift.
- Intro paragraph: `body.lg` (1–2 step bigger than body).
- Byline / deck / caption: `caption.sm` in `text.tertiary`.

## Serif / sans pairing
- Choose one of these pairings unless the DS declares its own:
  - Sans headline + Sans body (neutral-modern)
  - Serif headline + Sans body (editorial-classic)
  - Sans headline + Serif body (long-read)
- Do NOT mix Serif + Serif unless both come from the same superfamily.

## Rhythm devices
- Drop caps: allowed for the first paragraph of a section if the measure is ≥ 65 cpl and the text is long-form (>200 words in that section).
- Small caps for section labels — use `OpenType` small-caps, not uppercase-as-style.
- Hanging punctuation on pull quotes.

## Color roles
- Body text ≥ 85% darkness of primary.
- Captions and meta at 45–60% — keep them readable (contrast ≥ 4.5:1).
- Never use the accent color for the body of a paragraph. Accent lives in links, emphasis, or decorative headers.

## Emphasis
- Bold for critical scan-anchors; italics for titles and light emphasis. Do not mix both in the same sentence.
- Inline links: single underline on hover; body-weight; accent color.

## Hard fails (Plan lint)
- body column without `maxWidth` / measure control on an editorial surface → TYPO-EDITORIAL-001.
- drop cap inside a section whose body text is < 120 words → TYPO-EDITORIAL-002.
- accent color used for more than 1 paragraph of body → TYPO-EDITORIAL-003.
