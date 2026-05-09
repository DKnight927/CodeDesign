# Animation Discipline

> Motion is functional, not decorative. If you can't name the user-signal it carries, cut it.

## Budgets
- Micro-interaction (press, toggle, tooltip): 80–150ms, ease-out.
- Transition (modal, drawer, route): 200–250ms, ease-in-out.
- Choreography (list stagger, hero intro): total ≤ 400ms, per-item offset ≤ 40ms.
- Never exceed 300ms for a non-onboarding interaction.

## Figma representation
- Animations live in `states` or `annotations`, not in token values.
- Reference easing by name (`motion.ease.standard`, `motion.ease.decelerate`) — never inline bezier curves in multiple places.
- Each animated component gets a `motion` prop on its state frame describing trigger, duration, easing, properties.

## Forbidden patterns
- Bounce/overshoot on productivity surfaces (tables, forms, dashboards).
- Skeuomorphic flips, 3D rotations, confetti unless the user-signal is celebration AND it is dismissible AND it respects reduced-motion.
- "Staggered everything": decorative stagger on items that are not conceptually a list.
- Infinite loops on surfaces that stay mounted (CPU + distraction).

## Reduced motion
- Every animation MUST have a reduced-motion equivalent declared:
  - translation → instantaneous state swap
  - opacity crossfade ≤ 100ms is acceptable baseline
  - easing that shortens duration to ≤ 50ms is acceptable baseline

## Hard fails (Plan lint)
- motion prop with `duration > 300` on a non-onboarding step → MOTION-001.
- animation without a reduced-motion fallback declared → MOTION-002.
- more than 3 distinct easings referenced in a single screen → MOTION-003.
