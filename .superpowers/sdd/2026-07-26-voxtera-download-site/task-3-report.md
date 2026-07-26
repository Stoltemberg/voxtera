# Task 3 review

## Spec Compliance

**Not compliant.** The page implements the required content structure and download route, uses only the two project-owned `/images/` assets, preserves the download/API implementation, and does not touch launcher or obsolete-worktree files. However, it does not deliver the approved Adventure at Dawn design represented by the required visual reference.

## Strengths

- The exact primary CTA text appears in both download locations and both actions use `DOWNLOAD_URL`, which resolves to `/download` (`site/src/App.tsx:53`, `site/src/App.tsx:105`, `site/src/download.ts:1`).
- Required navigation targets, headings, feature labels, steps, semantic landmarks, Portuguese metadata, responsive layout, and reduced-motion rule are present.
- The test is focused on the critical downloadable launcher action, and the Vitest environment is switched to jsdom.
- Changed source files stay within the site; no launcher or API implementation changes appear in the supplied diff.

## Issues

### P1 — Approved visual reference is not reproduced

`site/src/App.tsx:59`, `site/src/App.tsx:76`, `site/src/App.tsx:83`; `site/src/styles.css:48-76`

The reference calls for the illustrated alternating feature rows (each with its own game image and icon), a centered three-column illustrated start sequence, a dark illustrated closing download band, and the wood-textured multi-column footer. The implementation substitutes a text-only two-column feature list, a separate repeated full-width use of the same snowy image, a text-only vertical start list, a plain meadow band, and a minimal footer. The hero also uses a solid paper header rather than the reference's overlaid transparent header. These are major composition and visual-system differences, so the page cannot be accepted as the approved landing page.

### P2 — Test dependency resolution is not supported on Node 18–21

`site/package.json:14`; `site/pnpm-lock.yaml` entry for `@testing-library/jest-dom@6.10.0`

The `^6.6.3` range resolves to `@testing-library/jest-dom@6.10.0`. Its lockfile metadata declares Node `>=22` and marks this release deprecated for a breaking change. Projects using the still-common Node 18/20 runtime can therefore fail or warn during install/test. Pinning a compatible release (for example 6.9.1) or explicitly requiring Node 22+ is needed before the required `npm test` result is dependable.

## Task-quality verdict

**Changes requested.** Preserve the working CTA/API pieces, but revise the visual structure to match the approved reference and resolve the test-runtime compatibility risk.

---

## Fix round — approved composition and test compatibility

### Changes made

- Rebuilt the page composition around the accepted Adventure at Dawn reference: transparent hero overlay navigation, alternating illustrated gameplay feature rows, a centered three-column start sequence with code-native icons, an illustrated dark closing download band, and a wood-toned four-column footer.
- Copied three real project-owned gameplay images from `assets/voxygen/background/` into `site/public/images/` with their source `.jpg` formats preserved: `forest-dawn.jpg`, `mountain-valley.jpg`, and `ruins-adventure.jpg`.
- Preserved all `DOWNLOAD_URL` CTA behavior and the exact primary CTA copy. The UI test now verifies both required primary CTA instances point to `/download`.
- Pinned `@testing-library/jest-dom` to compatible `6.9.1` and refreshed `pnpm-lock.yaml`.

### Verification

| Command | Result |
| --- | --- |
| `pnpm test -- App.test.tsx release.test.ts` | Passed: 2 files, 4 tests. |
| `pnpm run build` | Passed: TypeScript compilation and Vite production build. |

The forced `pnpm install --force --no-frozen-lockfile` completed successfully after the pin and repaired the missing-package-file state from the first implementation pass.
