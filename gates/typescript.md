# TypeScript / JavaScript Coding Gates

Mandatory on every TS / JS surface in the PlausiDen ecosystem.

## Project-level

- `tsconfig.json`: `"strict": true`, `"noUncheckedIndexedAccess": true`,
  `"noImplicitOverride": true`, `"exactOptionalPropertyTypes": true`.
- ESLint: `@typescript-eslint/recommended-type-checked` + project rules.
- Prettier with the repo-standard config; `prettier --check` runs in CI.

## Types

- No `any`. Use `unknown` and narrow.
- No type assertions (`as Foo`) without a `// SAFETY:` justification.
- No non-null assertions (`x!`) without justification.
- Branded types for domain primitives (`UserId`, `OrderId`) instead of
  bare strings.

## React (where applicable)

- Functional components with hooks; no class components.
- Strict React mode in dev.
- `useMemo` / `useCallback` only when profiled to matter.
- Explicit dependency arrays; ESLint `react-hooks/exhaustive-deps` enabled.
- Components ship with both light and dark mode tested.

## Async

- No floating promises. ESLint `@typescript-eslint/no-floating-promises`
  enforced.
- Every `await` has a timeout or a deliberate rationale.
- AbortController on every fetch.

## Dependencies

- `npm audit --omit=dev` (or `pnpm audit`) clean.
- No deps with known CVEs without an explicit `SHIP-DECISION:`.
- Lockfile (`package-lock.json` / `pnpm-lock.yaml`) committed.
- No deps that pull more than 100 transitive deps without justification.

## Build / test

- `tsc --noEmit` clean.
- `eslint .` clean (no warnings).
- `vitest` / `jest` / `playwright` green.
- Lighthouse / axe budget enforced on changed pages.

## Annotations

- `BUG ASSUMPTION:` on every exported function.
- `SECURITY:` on every defense-in-depth.
- `UX-DEBT:` on every manual-verification gap.

See [`../annotations/README.md`](../annotations/README.md).
