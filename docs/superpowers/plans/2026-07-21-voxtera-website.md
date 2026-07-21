# Voxtera Website and Vercel Publication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and publish an elegant, responsive Voxtera website whose primary CTA resolves to the exact launcher installer from the newest valid Preview release.

**Architecture:** A standalone Next.js 16 App Router project under `website/` renders a mostly server-side single-page experience from typed local content and shared root brand assets. A cached Route Handler queries public GitHub releases, validates the exact Preview asset contract, and returns a temporary redirect to GitHub without proxying the binary through Vercel.

**Tech Stack:** Next.js 16.2, React 19.2, TypeScript 7, CSS Modules and global CSS, Vitest 4, Testing Library 16, Zod 4.4, Playwright 1.61, axe-core 4.12, pnpm 11, Vercel CLI 56.4.

## Global Constraints

- Start only after the launcher plan publishes a verified `VoxteraLauncher-setup.exe` Preview asset.
- The site language is Portuguese for the first release.
- Use the approved cinematic voxel-fantasy identity: ice blue, restrained warm gold, dark stone/metal, real Voxtera captures or original project art.
- Do not use stock fantasy imagery, unrelated Veloren promotional art, autoplay video, or continuous heavy animation.
- Preserve exact primary copy and CTA labels defined in this plan.
- The launcher binary remains hosted by GitHub Releases, never proxied or stored by Vercel.
- `/api/download/windows` selects the newest non-draft release, including pre-releases, with exactly one `VoxteraLauncher-setup.exe` asset.
- Cache the resolved release for 300 seconds and return an explicit service error for broken contracts.
- Minimum requirements must match the official base-game requirements: Windows 10/11, 64-bit CPU, 4 GiB RAM, DirectX 12-or-newer or Vulkan-capable GPU, and 2 GiB available storage; Voxtera package size overrides the storage number if larger.
- Meet keyboard, focus, contrast, reduced-motion, responsive-image, and semantic HTML requirements.
- Target mobile Lighthouse performance at least 90 and LCP below 2.5 seconds under the recorded throttled profile.
- Verify a preview deployment before promoting to Vercel production.
- Never commit `.vercel`, tokens, credentials, or deployment environment files.

---

### Task 1: Next.js Project, Shared Brand Import, and Test Foundation

**Files:**
- Modify: `.gitignore`
- Create: `website/package.json`
- Create: `website/pnpm-lock.yaml`
- Create: `website/tsconfig.json`
- Create: `website/next.config.ts`
- Create: `website/eslint.config.mjs`
- Create: `website/vitest.config.ts`
- Create: `website/playwright.config.ts`
- Create: `website/src/test/setup.ts`
- Create: `website/src/app/layout.tsx`
- Create: `website/src/app/page.tsx`
- Create: `website/src/app/globals.css`
- Create: `website/src/app/page.test.tsx`
- Create: `website/src/lib/brand.ts`

**Interfaces:**
- Consumes: `brand/tokens.json` and approved `brand/` assets from the launcher plan.
- Produces: `pnpm test`, `pnpm lint`, `pnpm typecheck`, `pnpm build`, and a server-rendered home route.

- [ ] **Step 1: Write the failing home-page smoke test**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import Home from './page';

describe('Voxtera home', () => {
  it('renders the approved hero and download action', () => {
    render(<Home />);
    expect(screen.getByRole('heading', { name: 'Seu próximo mundo começa em blocos.' })).toBeVisible();
    expect(screen.getByRole('link', { name: 'Baixar launcher' })).toHaveAttribute('href', '/api/download/windows');
  });
});
```

- [ ] **Step 2: Run the test to verify RED**

Run:

```powershell
Set-Location website
pnpm test -- --run src/app/page.test.tsx
```

Expected: FAIL because the project and page do not exist.

- [ ] **Step 3: Create the pinned project configuration**

Create `website/package.json`:

```json
{
  "name": "voxtera-website",
  "private": true,
  "version": "0.1.0",
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "eslint . --max-warnings=0",
    "typecheck": "tsc --noEmit",
    "test": "vitest",
    "test:e2e": "playwright test"
  },
  "dependencies": {
    "lucide-react": "1.25.0",
    "next": "16.2.11",
    "react": "19.2.8",
    "react-dom": "19.2.8",
    "zod": "4.4.3"
  },
  "devDependencies": {
    "@playwright/test": "1.61.1",
    "@testing-library/jest-dom": "7.0.0",
    "@testing-library/react": "16.3.2",
    "@types/react": "19.2.17",
    "@types/react-dom": "19.2.3",
    "axe-core": "4.12.1",
    "eslint": "10.7.0",
    "eslint-config-next": "16.2.11",
    "jsdom": "29.1.1",
    "typescript": "7.0.2",
    "vitest": "4.1.10"
  },
  "packageManager": "pnpm@11.9.0"
}
```

Run `pnpm install --frozen-lockfile=false` and commit the generated lockfile. Do not enable analytics until privacy copy explicitly documents it; remove `@vercel/analytics` if unused.

- [ ] **Step 4: Configure safe monorepo asset access**

`next.config.ts` sets the repository root for tracing and allows only imported files under `brand/`. `brand.ts` parses `tokens.json` at build time and exposes typed values. Add a test that fails when required token names or approved brand image files are missing.

- [ ] **Step 5: Implement the minimal server-rendered page**

Create the tested heading and CTA with semantic `main`, `h1`, and link elements. Add metadata title `Voxtera — RPG voxel de aventura` and a concise Portuguese description. Avoid client components until interaction requires them.

- [ ] **Step 6: Verify the foundation**

Run:

```powershell
pnpm --dir website test -- --run
pnpm --dir website lint
pnpm --dir website typecheck
pnpm --dir website build
```

Expected: all tests PASS, zero lint warnings, typecheck exits 0, and Next production build succeeds.

- [ ] **Step 7: Commit the website foundation**

```powershell
git add .gitignore website
git commit -m "feat: scaffold Voxtera website"
```

---

### Task 2: Cached Preview Release Resolver and Direct Download Route

**Files:**
- Create: `website/src/lib/releases.ts`
- Create: `website/src/lib/releases.test.ts`
- Create: `website/src/test/fixtures/releases-valid.json`
- Create: `website/src/test/fixtures/releases-duplicate.json`
- Create: `website/src/test/fixtures/releases-broken.json`
- Create: `website/src/app/api/download/windows/route.ts`
- Create: `website/src/app/api/download/windows/route.test.ts`
- Create: `website/src/components/DownloadButton.tsx`
- Create: `website/src/components/DownloadButton.test.tsx`
- Modify: `website/src/app/page.tsx`

**Interfaces:**
- Consumes: GitHub public releases API and exact `VoxteraLauncher-setup.exe` release asset.
- Produces: `resolveLatestPreview(fetcher) -> Promise<LauncherRelease>`, GET `/api/download/windows`, and visible resolved version/platform copy.

- [ ] **Step 1: Write failing release resolver tests**

```ts
it('selects the newest non-draft release including prereleases', async () => {
  const release = await resolveLatestPreview(fetchFixture('releases-valid.json'));
  expect(release.tag).toBe('v0.3.0-preview.1');
  expect(release.asset.name).toBe('VoxteraLauncher-setup.exe');
});

it('rejects duplicate exact launcher assets', async () => {
  await expect(resolveLatestPreview(fetchFixture('releases-duplicate.json')))
    .rejects.toMatchObject({ code: 'release_contract' });
});
```

Cover drafts, unsorted versions, malformed tags, missing asset, duplicate asset, non-HTTPS URL, and GitHub non-200/rate-limit responses.

- [ ] **Step 2: Write failing route tests**

Inject the resolver. Assert success returns `307`, exact immutable GitHub `Location`, `Cache-Control: public, s-maxage=300, stale-while-revalidate=600`, and `X-Voxtera-Version`. Broken contracts return JSON `503` with `Retry-After`, never a GitHub releases-page redirect.

- [ ] **Step 3: Run resolver and route tests to verify RED**

Run `pnpm --dir website test -- --run src/lib/releases.test.ts src/app/api/download/windows/route.test.ts`.

Expected: FAIL because resolver and route do not exist.

- [ ] **Step 4: Implement validated GitHub release parsing**

Use Zod schemas that retain only `tag_name`, `draft`, `prerelease`, and asset `name`, `size`, `browser_download_url`. Send `Accept: application/vnd.github+json`, `X-GitHub-Api-Version`, and a Voxtera user agent. Use Next server fetch caching with `next: { revalidate: 300 }`. Sort parsed semantic versions, not API order.

- [ ] **Step 5: Implement the redirect route and download CTA**

Use `NextResponse.redirect(validatedUrl, 307)` and set cache/version headers. `DownloadButton` links to the route with visible subcopy `Windows 10/11 • 64 bits • Preview <version>` when server-rendered metadata is available, otherwise `Windows 10/11 • 64 bits`.

- [ ] **Step 6: Verify route behavior without downloading the executable**

Run focused tests, then start the site and request the route with redirects disabled:

```powershell
pnpm --dir website test -- --run
pnpm --dir website build
curl.exe -I --max-redirs 0 http://localhost:3000/api/download/windows
```

Expected: tests PASS; response is 307 with an exact GitHub asset URL. Do not follow the redirect during this check.

- [ ] **Step 7: Commit download resolution**

```powershell
git add website/src
git commit -m "feat: resolve latest Voxtera launcher download"
```

---

### Task 3: Approved Content, Cinematic Layout, and Original Visual Assets

**Files:**
- Create: `website/src/content/site.ts`
- Create: `website/src/components/SiteHeader.tsx`
- Create: `website/src/components/Hero.tsx`
- Create: `website/src/components/WorldSection.tsx`
- Create: `website/src/components/Gallery.tsx`
- Create: `website/src/components/LauncherSection.tsx`
- Create: `website/src/components/Requirements.tsx`
- Create: `website/src/components/FinalCta.tsx`
- Create: `website/src/components/SiteFooter.tsx`
- Create: `website/src/components/sections.test.tsx`
- Create: `website/public/images/world-hero.avif`
- Create: `website/public/images/world-hero.webp`
- Create: `website/public/images/gallery-01.avif`
- Create: `website/public/images/gallery-02.avif`
- Create: `website/public/images/gallery-03.avif`
- Modify: `website/src/app/page.tsx`
- Modify: `website/src/app/globals.css`

**Interfaces:**
- Consumes: approved launcher identity, real Voxtera captures or original project artwork, and official base-game minimum requirements.
- Produces: complete single-page content in the approved order and exact above-the-fold copy.

- [ ] **Step 1: Lock the visible copy inventory in tests**

The above-the-fold inventory is exactly:

```ts
export const heroCopy = {
  navigation: ['O mundo', 'Launcher', 'Requisitos'],
  heading: 'Seu próximo mundo começa em blocos.',
  body: 'Explore um RPG voxel de aventura, combates e descobertas em um mundo vivo.',
  primaryCta: 'Baixar launcher',
  secondaryCta: 'Conhecer o mundo',
  platform: 'Windows 10/11 • 64 bits',
} as const;
```

Write a test that rejects added hero badges, eyebrows, promotional claims, or renamed CTA copy.

- [ ] **Step 2: Create and inspect the visual asset set**

Use real Voxtera screenshots where they present the current product well. For missing hero composition, generate original voxel-fantasy landscape artwork consistent with Voxtera, without text or third-party characters. Preserve a dark negative-space region behind hero copy. Convert approved masters to AVIF/WebP and inspect every final file with `view_image`.

- [ ] **Step 3: Write failing section and image tests**

Assert section order, heading hierarchy, launcher CTA targets, gallery alt text, image dimensions, requirements copy, credit/license presence, and no missing local asset. Ensure the hero uses `next/image` with `priority` and responsive `sizes`.

- [ ] **Step 4: Run section tests to verify RED**

Run `pnpm --dir website test -- --run src/components/sections.test.tsx`.

Expected: FAIL because sections/content/assets do not exist.

- [ ] **Step 5: Implement the approved landing page**

Use one clear content rail with sharp crafted panels and controlled overlap between the hero and next-section preview. World features are limited to accurate claims: open voxel world, action combat/progression, multiplayer adventure, and Voxtera's Portuguese quest/dialogue work. Do not claim building, commerce, account persistence, or features not verified in the repository.

Requirements show the sourced minimum and a labeled recommended target of Windows 11 x64, 16 GiB RAM, modern 4 GB GPU with DX12/Vulkan, and free storage equal to at least twice the current archive size for staging. The archive-derived storage value comes from release metadata rather than a hardcoded number.

- [ ] **Step 6: Verify content and production build**

Run:

```powershell
pnpm --dir website test -- --run
pnpm --dir website lint
pnpm --dir website typecheck
pnpm --dir website build
```

Expected: tests PASS and no broken local image appears in build output.

- [ ] **Step 7: Commit content and composition**

```powershell
git add website brand
git commit -m "feat: build cinematic Voxtera landing page"
```

---

### Task 4: Responsive Behavior, Accessibility, and Meaningful Interaction

**Files:**
- Create: `website/src/components/MobileNav.tsx`
- Create: `website/src/components/GalleryCarousel.tsx`
- Create: `website/src/components/DownloadStatus.tsx`
- Create: `website/src/components/accessibility.test.tsx`
- Create: `website/tests/responsive.spec.ts`
- Create: `website/tests/download.spec.ts`
- Modify: `website/src/app/globals.css`
- Modify: `website/src/components/SiteHeader.tsx`
- Modify: `website/src/components/Hero.tsx`
- Modify: `website/src/components/Gallery.tsx`
- Modify: `website/src/components/LauncherSection.tsx`
- Modify: `website/src/components/FinalCta.tsx`
- Modify: `website/src/components/SiteFooter.tsx`

**Interfaces:**
- Consumes: complete page from Task 3 and download route from Task 2.
- Produces: keyboard/mobile navigation, functional gallery, live download-state feedback, and Playwright flows at desktop and mobile viewports.

- [ ] **Step 1: Write failing accessibility and responsive tests**

Use axe-core against the rendered page. Assert one `h1`, named regions, visible keyboard focus, menu state and `aria-expanded`, carousel button names, reduced-motion CSS, 44 px minimum touch targets, and no horizontal overflow at 390x844.

- [ ] **Step 2: Write failing browser workflow tests**

At 1440x900 and 390x844, verify header navigation scrolls to exact sections, mobile menu opens/closes and restores focus, gallery controls change the visible image, CTA request receives a redirect without downloading the asset, and broken release contract presents a retryable status.

- [ ] **Step 3: Run tests to verify RED**

Run:

```powershell
pnpm --dir website test -- --run src/components/accessibility.test.tsx
pnpm --dir website test:e2e -- tests/responsive.spec.ts tests/download.spec.ts
```

Expected: component and browser tests FAIL until responsive interactions exist.

- [ ] **Step 4: Implement accessible interactions**

Keep the page server-first. Only mobile navigation, carousel controls, and download status are client components. Use real buttons and SVG icons, escape closes overlays, focus returns to the trigger, and all animation is removed under `prefers-reduced-motion: reduce`.

- [ ] **Step 5: Verify desktop and mobile flows**

Run the focused tests, then the full unit/E2E suites. Capture desktop first viewport, desktop full page, and mobile full page. Inspect the screenshots with `view_image`; fix overflow, crop, focus, typography, and contrast issues.

- [ ] **Step 6: Commit responsive and accessible behavior**

```powershell
git add website/src website/tests website/playwright.config.ts
git commit -m "feat: make Voxtera site responsive and accessible"
```

---

### Task 5: Fidelity, Performance, Security, and Production Build Gate

**Files:**
- Create: `website/QA.md`
- Create: `website/SECURITY.md`
- Create: `website/tests/production.spec.ts`
- Modify: `website/next.config.ts`
- Modify: `website/src/app/layout.tsx`
- Modify: `website/src/app/globals.css`

**Interfaces:**
- Consumes: final local site and accepted visual direction.
- Produces: agency-signoff fidelity ledger, security headers, local production smoke tests, and measured performance report.

- [ ] **Step 1: Add failing production-header tests**

Assert CSP suitable for self-hosted assets plus GitHub API/redirect, `Referrer-Policy`, `X-Content-Type-Options`, frame denial, permissions policy, no source maps in production responses, and no secret-bearing client environment variables.

- [ ] **Step 2: Implement production headers and metadata**

Configure exact metadata, Open Graph image from approved Voxtera art, a canonical origin derived at deployment from `VERCEL_PROJECT_PRODUCTION_URL`, robots, sitemap, and security headers. Do not add third-party analytics, fonts, trackers, or scripts.

- [ ] **Step 3: Perform visual fidelity QA**

Compare the approved art direction and final screenshots at 1440x900 and 390x844. In `QA.md`, record at least five points: hero balance, next-section visibility, typography, palette, asset blending, CTA/icon fidelity, responsive behavior, and motion. Record the above-the-fold copy diff; it must be empty.

- [ ] **Step 4: Measure production performance**

Run a local production server and Lighthouse against desktop and mobile. Record score, LCP, CLS, image payload, JS payload, and server-route latency. Fix all material misses. If the mobile score remains below 90 or LCP exceeds 2.5 seconds, the task remains incomplete unless the user explicitly accepts the measured variance.

- [ ] **Step 5: Run the complete local gate**

```powershell
pnpm --dir website test -- --run
pnpm --dir website lint
pnpm --dir website typecheck
pnpm --dir website build
pnpm --dir website test:e2e
git diff --check
git status --short
```

Expected: every command exits 0; only planned QA/security files are uncommitted before the task commit.

- [ ] **Step 6: Commit production readiness**

```powershell
git add website
git commit -m "perf: harden Voxtera website for production"
```

---

### Task 6: Vercel Preview, Browser Verification, Promotion, and Share Link

**Files:**
- Create locally but never commit: `website/.vercel/`
- Modify: `website/QA.md` with deployment identifiers and final verification evidence.

**Interfaces:**
- Consumes: Vercel-authenticated CLI session, production-ready `website/`, and live GitHub launcher Preview.
- Produces: verified Vercel preview URL, promoted production URL, working direct launcher download, and post-deploy error scan.

- [ ] **Step 1: Confirm deployment prerequisites without changing production**

Run:

```powershell
pnpm dlx vercel@56.4.1 whoami
git status --short
git check-ignore website/.vercel
```

Expected: authenticated account name, no unexpected workspace changes, and `.vercel` is ignored. If authentication is absent, stop for `vercel login`; do not invent or request a token in chat.

- [ ] **Step 2: Link the monorepo project to `website/`**

From the repository root, link or create project `voxtera` with Root Directory `website`. Confirm framework Next.js and no secret environment variables. Do not connect the game build or launcher directories to Vercel.

Run:

```powershell
pnpm dlx vercel@56.4.1 link --cwd website --yes --project voxtera
```

- [ ] **Step 3: Create a preview deployment**

Run:

```powershell
pnpm dlx vercel@56.4.1 deploy --cwd website --yes
```

Capture the deployment URL and wait until `vercel inspect <url>` reports `READY`. Do not promote yet.

- [ ] **Step 4: Verify the preview in a real browser**

Use Browser/IAB first. Inspect the first viewport, scroll every section, test desktop and mobile sizes, exercise nav/gallery, and activate the download CTA with network inspection that confirms the exact GitHub launcher asset. Capture final screenshots and inspect them with `view_image` alongside local QA screenshots.

Run remote Playwright smoke tests with `PLAYWRIGHT_BASE_URL=<preview-url>` and verify status, metadata, images, and redirect contract. Scan `vercel logs <url> --level error`.

- [ ] **Step 5: Promote the verified deployment**

Run:

```powershell
pnpm dlx vercel@56.4.1 promote <preview-url> --yes --timeout 5m
pnpm dlx vercel@56.4.1 promote status
```

Expected: promotion completes and a production URL is assigned. If Vercel rebuilds for production environment, wait for the resulting production deployment to reach `READY` and verify it separately.

- [ ] **Step 6: Verify production and the downloadable launcher**

Check production root, sitemap, robots, all local assets, and `/api/download/windows`. Confirm the redirect selects the exact current Preview installer and that the asset begins downloading; cancel after headers and initial bytes to avoid an unnecessary repeated full download.

Run:

```powershell
pnpm dlx vercel@56.4.1 logs --environment production --level error --since 10m
```

Expected: no unresolved application errors.

- [ ] **Step 7: Record evidence and commit only repository documentation**

Append preview deployment ID, production deployment ID, production URL, verification timestamps, screenshot paths, download asset name/version, and log scan result to `website/QA.md`. Confirm `website/.vercel` remains untracked.

```powershell
git add website/QA.md
git commit -m "docs: record Voxtera production deployment"
```

- [ ] **Step 8: Final handoff**

Report the shareable production URL, `READY` status, Git commit, framework, build duration, download asset/version, browser viewports checked, performance results, error-log scan, and any intentional variance. Do not call the site published until the production URL and download route were both tested after promotion.
