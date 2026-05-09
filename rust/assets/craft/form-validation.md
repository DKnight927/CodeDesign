# Form Validation

> Forms are contracts. Tell users what you need, when you need it, and how to fix what's wrong.

## Field-level
- Required indicator on the LABEL, not after the input.
- Help text (`caption.sm`, `text.secondary`) between label and input when needed.
- Error state: border `state.danger`, error message below input in `caption.sm` `text.danger`. Message starts with the field name, explains the problem, and names the fix. Bad: "Invalid". Good: "Email needs an @ — e.g. you@example.com".
- Success state is used sparingly: only for async validation wins (e.g., "username available"), not as default.

## Timing
- Validate on blur for correctness (format, existence).
- Validate on submit for cross-field rules.
- Never validate on every keystroke except for real-time async checks (username availability, password strength).
- Debounce async validation ≥ 300ms.

## Password
- Show password toggle is required.
- Strength meter uses 3–4 discrete levels, labeled (weak / fair / strong). No false-precision percentages.
- Rules visible BEFORE the user types, checked inline as they type.

## Inline vs summary
- Inline error at each field is mandatory.
- Summary at top of form appears only when form length ≥ 5 fields OR server returns a whole-form error.
- Summary links to its field via anchor.

## Submit
- Submit button disabled only when the form is BOTH untouched AND invalid; never gray out a filled form — let the user see the specific errors on click.
- Loading state after click: label → spinner + "Saving…". No duplicate submissions.
- Success state: inline confirmation that names what changed; no modal.

## Figma representation
- Every input component has states: `default, hover, focus, filled, error, disabled, readonly`.
- Error variant references `color.state.danger` and shows a representative message. Do not leave error messages as lorem.

## Hard fails (Plan lint)
- input instance without `error` state present in component set → FORM-001.
- error message literal is lorem / generic ("Invalid", "Error") → FORM-002.
- password input without show-password toggle → FORM-003.
- submit disabled only with no reason shown → FORM-004.
