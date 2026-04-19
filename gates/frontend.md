# Frontend (HTML / CSS) Coding Gates

Applies to every user-facing surface — dashboards, the Classroom, marketing
pages, in-product modals, the lot.

## HTML

- Semantic elements first. `div` only when no semantic element fits.
- One `<h1>` per page. Heading hierarchy preserved (no skipping levels).
- Every form control has an associated `<label>`.
- Every image has `alt` text (or `alt=""` for decorative).
- Every interactive element reachable by keyboard.

## CSS

- Design tokens only (`tokens.css` / theme JSON). No `#hex` literals
  outside the token file.
- Spacing on a 4 px grid (8 px preferred). No arbitrary `margin: 13px`.
- Typography uses the defined scale. No arbitrary `font-size`.
- Dark mode + light mode both shipped, both tested.
- `prefers-reduced-motion` respected.

## Accessibility (gates `accessibility` audit)

- WCAG 2.1 AA contrast ratios verified by axe / Lighthouse.
- All interactive elements have visible focus indicators.
- ARIA roles only when native semantics are insufficient.
- No keyboard traps; `Esc` cancels modals; `Tab` order matches visual order.

## Performance

- Critical CSS inlined; rest deferred.
- Images responsive (`srcset`, `sizes`) and lazy-loaded below the fold.
- Fonts subset and preloaded; no FOIT.
- Third-party scripts deferred; none added without a `SHIP-DECISION:`.
- Bundle size budget per route enforced in CI.

## States (gates `ui` audit)

- Every data view has designed empty / loading / error / partial states.
- Toast notifications announced (`aria-live="polite"`).
- Modals trap focus and restore on close.

## Annotations

- `UX-DEBT:` for any manual UX verification gap.
- `BUG ASSUMPTION:` on every component prop interface.

See [`../annotations/README.md`](../annotations/README.md).
