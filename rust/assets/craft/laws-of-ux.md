# Laws of UX (applied to Figma output)

> CodeDesign is evaluated against these; cite them by name when explaining Plan choices.

## Aesthetic-Usability Effect
Good-looking product feels easier to use. Craft compounds — do not defer typography / spacing / color.

## Hick's Law
Choices stack decision cost. If a screen offers > 7 primary actions, split it or rank visibly.

## Fitts's Law
Important targets are big and close. Primary CTA ≥ 44px tall, within thumb zone on mobile, anchored bottom-right on desktop modals.

## Miller's Law
Short-term recall ≈ 7±2. Group inputs and navigation into chunks of 3–5.

## Jakob's Law
Users apply convention from other products. Do NOT reinvent the search input, modal close, tab bar, settings entry without a strong reason.

## Doherty Threshold
Perceived responsiveness under 400ms keeps flow. Skeletons or optimistic UI if server > 400ms. No blocking spinner for < 400ms.

## Peak-End Rule
A session is remembered by its peak moment and its end. Empty states and success states deserve the same care as the core flow.

## Serial Position Effect
First and last items get the most attention. Put the most-used nav items at ends, not middle.

## Law of Proximity / Common Region
Closer things are read as grouped. Lean on spacing before lean on borders.

## Law of Uniform Connectedness
Connected elements (same fill, shared container) are perceived as a unit. Prefer surface over line.

## Plan-level asserts
- Primary CTA in hero: 1 per screen, named in rationale.
- Nav item count ≤ 7 at any level.
- Skeleton state present for any screen whose main content is server-fetched.
- Empty state is NEVER a blank frame; name the user's likely next action.
