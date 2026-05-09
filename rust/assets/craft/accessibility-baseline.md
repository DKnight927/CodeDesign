# Accessibility Baseline

> CodeDesign's non-negotiable a11y floor. Violations block Plan emission.

## Contrast (text on fill)
- Body text ≥ 4.5:1 against its computed background.
- Large text (≥ 18pt regular / 14pt bold) ≥ 3:1.
- Icons used as sole cue for state: ≥ 3:1 against fill.
- Figma enforcement: every `text` node references `color.text.*` token; never raw hex. Lint recomputes contrast off the token graph before commit.

## Focus visibility
- Every interactive instance (button, input, nav.item, link) MUST have a focus ring state documented in the component set.
- Focus ring: solid stroke, ≥ 2px, offset ≥ 2px, contrast ≥ 3:1 against both the control and its background.
- Do NOT rely on color change alone to signal focus.

## Hit target
- Touch targets ≥ 44×44 px (mobile) / ≥ 32×32 px (desktop pointer).
- Adjacent targets need ≥ 8px gap.

## Keyboard
- Every screen Plan MUST name the tab order explicitly in `states` or `annotations` for non-trivial flows.
- Modals MUST trap focus and restore it on close.

## Motion
- Any animation > 200ms or involving translation > 20px must respect reduced-motion (noted in `states`).
- No strobe-like flicker; flashing ≤ 3Hz.

## Language & RTL
- See `rtl-and-bidi.md`.
- Every text layer has a language identifier if the artboard supports more than one locale.

## Alt text / labels
- Every non-decorative image frame has an `alt` prop.
- Icon-only buttons have a visible label via tooltip prop OR an explicit `ariaLabel` prop on the component instance.

## Hard fails (Plan lint)
- text node with `fillRef` ending in `.tertiary` + surface `bg.sunken` → always fails contrast; lint rejects with code A11Y-001.
- interactive instance without a state entry covering `hover`, `focus`, `disabled` → code A11Y-002.
- touch target < 44×44 on `platform: mobile` → code A11Y-003.
