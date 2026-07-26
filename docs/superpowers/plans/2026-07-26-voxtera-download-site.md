# Voxtera Download Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and deploy a bright, welcoming Voxtera landing page that sends Windows players to the current `VoxteraLauncher.exe` release.

**Architecture:** Create an isolated Vite + React + TypeScript app in `site/`. The client renders a responsive image-led landing page and targets `/download`; a Vercel function resolves the latest GitHub release asset named `VoxteraLauncher.exe` and redirects to it, falling back to the releases page when no matching asset is found.

**Tech Stack:** React 19, TypeScript, Vite, CSS, Vitest, Vercel Functions, Vercel CLI.

## Global Constraints

- Keep the retained Python launcher untouched: `launcher/voxtera_launcher.py` and its packaged executable remain in place.
- Remove only the obsolete Tauri worktree at `.worktrees/launcher-site` after site verification.
- Reuse project-owned Voxtera/game imagery; persist all web assets under `site/public/`.
- Use the exact primary CTA label `Baixar launcher para Windows (.exe)`.
- The download path must resolve the latest `VoxteraLauncher.exe` release asset, with the GitHub releases page as a failure fallback.
- Verify desktop and mobile rendering before production deploy.

---

## Planned file structure

- `site/package.json` — Vite scripts and frontend/test dependencies.
- `site/src/main.tsx` — React entry point.
- `site/src/App.tsx` — page composition and content hierarchy.
- `site/src/styles.css` — design tokens, responsive layout, visual treatment, and reduced-motion rules.
- `site/public/images/` — repository-owned logo and gameplay capture copied for deployment.
- `site/src/download.ts` — client-facing `/download` URL constant.
- `site/src/release.ts` — pure latest-release asset-selection helper.
- `site/api/download.ts` — Vercel serverless redirect handler.
- `site/src/release.test.ts` — Vitest coverage for release selection and fallback target logic.
- `site/vercel.json` — explicit Vite build and output configuration.
- `site/README.md` — local development, test, build, and deployment instructions.

### Task 1: Scaffold the independent site and preserve web assets

**Files:**
- Create: `site/package.json`
- Create: `site/tsconfig.json`
- Create: `site/vite.config.ts`
- Create: `site/index.html`
- Create: `site/src/main.tsx`
- Create: `site/src/App.tsx`
- Create: `site/src/styles.css`
- Create: `site/public/images/voxtera-logo.png`
- Create: `site/public/images/gameplay-capture.png`
- Create: `site/README.md`

**Interfaces:**
- Consumes: repository-owned logo and approved gameplay imagery.
- Produces: `npm run dev`, `npm run build`, and `npm test` commands; an `App` component rendered by `main.tsx`.

- [ ] **Step 1: Inspect usable repository-owned image candidates without traversing `node_modules`**

Run:

```powershell
rg --files assets launcher -g '*.{png,jpg,jpeg,webp}'
```

Expected: a short candidate list including the Voxtera logo and any suitable high-resolution gameplay captures. Choose only images that visibly represent the game; do not use generated images or unlicensed stock art.

- [ ] **Step 2: Create the Vite project manifest and test configuration**

Create `site/package.json` with the commands below and the matching React, Vite, TypeScript and Vitest dev dependencies:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "test": "vitest run"
  }
}
```

Set `vite.config.ts` to `defineConfig({ test: { environment: "node" } })` and configure TypeScript to support React JSX and DOM types.

- [ ] **Step 3: Add the initial semantic app shell and prove the production build works**

Create the React entry point and a small composition shell:

```tsx
export function App() {
  return <main aria-label="Voxtera" />;
}
```

Copy the approved project-owned hero capture to `site/public/images/gameplay-capture.png` and the project logo to `site/public/images/voxtera-logo.png`, then import `styles.css` in `main.tsx`.

- [ ] **Step 4: Install dependencies and run the build**

Run:

```powershell
npm install
npm run build
```

Expected: `site/dist/` is created with no TypeScript or Vite errors.

- [ ] **Step 5: Commit the independently buildable scaffold**

```powershell
git add site
git commit -m "feat: scaffold Voxtera download site"
```

### Task 2: Add and test the latest-launcher download resolver

**Files:**
- Create: `site/src/download.ts`
- Create: `site/src/release.ts`
- Create: `site/src/release.test.ts`
- Create: `site/api/download.ts`
- Create: `site/vercel.json`

**Interfaces:**
- Consumes: GitHub's `GET /repos/Stoltemberg/voxtera/releases/latest` response with `html_url` and `assets[].{name,browser_download_url}`.
- Produces: `findLauncherUrl(release): string | null` and a `GET /api/download` redirect response.

- [ ] **Step 1: Write failing asset-selection tests**

Create `site/src/release.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { findLauncherUrl } from "./release";

describe("findLauncherUrl", () => {
  it("returns the exact Windows launcher executable", () => {
    expect(findLauncherUrl({
      assets: [
        { name: "game.zip", browser_download_url: "https://example.test/game.zip" },
        { name: "VoxteraLauncher.exe", browser_download_url: "https://example.test/launcher.exe" }
      ]
    })).toBe("https://example.test/launcher.exe");
  });

  it("returns null when the executable is absent", () => {
    expect(findLauncherUrl({ assets: [] })).toBeNull();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- release.test.ts`

Expected: FAIL because `./release` does not exist.

- [ ] **Step 3: Implement the strict asset selector and public route constant**

Create `site/src/release.ts`:

```ts
type ReleaseAsset = { name: string; browser_download_url: string };
type LatestRelease = { assets?: ReleaseAsset[] };

export function findLauncherUrl(release: LatestRelease): string | null {
  return release.assets?.find((asset) => asset.name === "VoxteraLauncher.exe")
    ?.browser_download_url ?? null;
}
```

Create `site/src/download.ts` with:

```ts
export const DOWNLOAD_URL = "/download";
```

Implement `site/api/download.ts` to fetch `https://api.github.com/repos/Stoltemberg/voxtera/releases/latest` with a `User-Agent: VoxteraDownloadSite` header, call `findLauncherUrl`, and send a 307 redirect to the selected asset. On a non-OK response, thrown fetch error, malformed payload, or missing asset, redirect with 302 to `https://github.com/Stoltemberg/voxtera/releases`.

Add `site/vercel.json`:

```json
{
  "buildCommand": "npm run build",
  "outputDirectory": "dist",
  "rewrites": [{ "source": "/download", "destination": "/api/download" }]
}
```

- [ ] **Step 4: Run tests and production build**

Run:

```powershell
npm test -- release.test.ts
npm run build
```

Expected: both release-selection tests pass and Vite emits `dist/`.

- [ ] **Step 5: Commit the tested download behavior**

```powershell
git add site/src/download.ts site/src/release.ts site/src/release.test.ts site/api/download.ts site/vercel.json
git commit -m "feat: resolve latest Voxtera launcher download"
```

### Task 3: Implement the approved Adventure at Dawn landing page

**Files:**
- Modify: `site/src/App.tsx`
- Modify: `site/src/styles.css`
- Modify: `site/index.html`

**Interfaces:**
- Consumes: `DOWNLOAD_URL` from `site/src/download.ts` and the assets from `/images/`.
- Produces: responsive anchor navigation to `#game` and `#start`, and all download actions target `/download`.

- [ ] **Step 1: Add a minimal content-structure test before visual implementation**

Create `site/src/App.test.tsx` using React Testing Library:

```tsx
it("exposes the primary executable download action", () => {
  render(<App />);
  expect(screen.getByRole("link", {
    name: "Baixar launcher para Windows (.exe)"
  })).toHaveAttribute("href", "/download");
});
```

Configure Vitest's `jsdom` environment and add `@testing-library/react` plus `@testing-library/jest-dom` as development dependencies.

- [ ] **Step 2: Run the page test to confirm the shell fails it**

Run: `npm test -- App.test.tsx`

Expected: FAIL because the initial app shell has no accessible download link.

- [ ] **Step 3: Compose the exact approved sections in `App.tsx`**

Implement semantic `header`, `main`, `section`, `footer`, headings, and descriptive image alt text. Include exactly these visible sections and primary-copy strings:

```tsx
<h1>Seu mundo voxel começa aqui</h1>
<a href={DOWNLOAD_URL}>Baixar launcher para Windows (.exe)</a>
<p>Windows 10/11 · instalação e atualizações automáticas</p>
<section id="game" aria-labelledby="game-title">
  <h2 id="game-title">Feito para explorar</h2>
</section>
<section id="start" aria-labelledby="start-title">
  <h2 id="start-title">Comece em minutos</h2>
</section>
```

The `#game` section must introduce `Explore`, `Construa`, and `Aventure-se`. The `#start` section must list `Baixe o launcher`, `Instale em poucos cliques`, and `Jogue`. Repeat the same `DOWNLOAD_URL` in the closing download band. Link the footer to the GitHub repository and releases history.

- [ ] **Step 4: Implement the design system and responsive treatment in `styles.css`**

Define CSS custom properties for sky, meadow, wood, ink, paper, and a green action color. Use a full-bleed hero image with only the approved legibility gradient; do not tint or obscure the image. At widths below `760px`, stack the navigation, tighten hero padding, present the experience narrative in one column, and keep all CTAs full-width and tap-friendly. Include:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { scroll-behavior: auto; transition-duration: 0.01ms; }
}
```

Set the page title and Portuguese metadata in `index.html`.

- [ ] **Step 5: Run frontend tests and build**

Run:

```powershell
npm test
npm run build
```

Expected: release and page tests pass, and the application builds without warnings.

- [ ] **Step 6: Commit the landing page**

```powershell
git add site/src/App.tsx site/src/App.test.tsx site/src/styles.css site/index.html site/package.json site/vite.config.ts
git commit -m "feat: build Voxtera adventure landing page"
```

### Task 4: Perform visual QA and remove the obsolete Tauri worktree

**Files:**
- Modify: `site/README.md`
- Delete: `.worktrees/launcher-site/` (only after all verification passes)

**Interfaces:**
- Consumes: production build output and the page's `/download` route.
- Produces: browser-verified desktop/mobile site and no Tauri worktree.

- [ ] **Step 1: Start the site locally and inspect the actual rendered page**

Run: `npm run dev -- --host 127.0.0.1`

Expected: Vite reports a local URL. Use the in-app browser to open it, check the hero, scroll through all sections, and activate a CTA to verify that it targets `/download`.

- [ ] **Step 2: Capture desktop and mobile evidence and perform a visual fidelity review**

Inspect both a desktop viewport (1440px wide) and a mobile viewport (390px wide). Capture browser screenshots and inspect them with `view_image` alongside the accepted visual concept. Check at least these five items: hero image treatment, bright sky/green/wood palette, heading scale, CTA visibility and wording, section order, and mobile overflow. Correct every material mismatch before proceeding.

- [ ] **Step 3: Verify the retained Python launcher before cleanup**

Run:

```powershell
Test-Path launcher\voxtera_launcher.py
Test-Path launcher\dist\VoxteraLauncher.exe
```

Expected: both commands return `True`.

- [ ] **Step 4: Remove the exact obsolete worktree only**

First inspect it:

```powershell
git -C .worktrees\launcher-site status --short
```

If it contains no uncommitted work that must be preserved, remove only this confirmed target using Git's worktree command when it is registered; otherwise remove the directory after confirming it is the Tauri worktree. Never target `launcher/` or the repository root.

- [ ] **Step 5: Verify cleanup and commit documentation updates**

Run:

```powershell
Test-Path .worktrees\launcher-site
Test-Path launcher\voxtera_launcher.py
Test-Path launcher\dist\VoxteraLauncher.exe
```

Expected: `False`, `True`, `True` in that order. Update `site/README.md` with actual local commands, then commit it:

```powershell
git add site/README.md
git commit -m "docs: document Voxtera site operations"
```

### Task 5: Deploy and smoke-test Vercel production

**Files:**
- Modify: `site/README.md`

**Interfaces:**
- Consumes: `site/vercel.json`, `site/dist/`, and logged-in Vercel CLI credentials.
- Produces: a public production URL and verified downloadable redirect.

- [ ] **Step 1: Deploy from the isolated site root**

Run:

```powershell
vercel --prod --yes
```

Expected: Vercel returns one production deployment URL. Record it in `site/README.md` under `Production URL`.

- [ ] **Step 2: Verify production behavior in a browser**

Open the production URL in the in-app browser. Check desktop and mobile layout, each navigation anchor, and both download buttons. Request `/download` and verify that it redirects to a URL ending in `VoxteraLauncher.exe`, or, if no current executable is published, to the GitHub releases page.

- [ ] **Step 3: Commit the final production documentation**

```powershell
git add site/README.md
git commit -m "docs: record Voxtera site deployment"
```

## Plan self-review

- **Spec coverage:** Task 1 handles isolated setup and owned assets; Task 2 implements current-release executable resolution plus fallback; Task 3 implements every approved section and copy string; Task 4 verifies responsive fidelity and safely removes only the Tauri worktree; Task 5 deploys and smoke-tests production.
- **Placeholder scan:** No `TBD`, `TODO`, or deferred implementation notes remain. The single selected gameplay asset is intentionally discovered in Task 1 because the repository currently has no documented screenshot inventory.
- **Interface consistency:** `DOWNLOAD_URL` is defined once in `site/src/download.ts`, the page and its test use `/download`, and `findLauncherUrl` is the sole pure selector consumed by the Vercel download handler.
