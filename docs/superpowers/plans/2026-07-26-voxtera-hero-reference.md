# Voxtera Hero Reference Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the published Voxtera hero match the supplied bright game-world reference while keeping the direct Windows launcher download intact.

**Architecture:** The supplied PNG becomes a static public asset consumed only by the existing `App` hero image. `App.tsx` keeps the semantic structure and download URL contract; `styles.css` changes the hero palette, gradients, sizing and responsive crop. The rest of the cinematic page is deliberately untouched.

**Tech Stack:** React, TypeScript, Vite, CSS, Vitest, Vercel static hosting.

## Global Constraints

- Use the user-provided PNG as the hero artwork rather than `forest-dawn.jpg`.
- Preserve the direct launcher URL `/downloads/VoxteraLauncher.exe` in every CTA.
- Add no GitHub link or external download redirect.
- Preserve the existing gameplay sections beneath the hero.
- Ensure desktop and mobile retain readable copy and a visible direct `.exe` download action.

---

### Task 1: Replace and style the hero artwork

**Files:**
- Create: `site/public/images/voxtera-hero-reference.png`
- Modify: `site/src/App.tsx:10-31`
- Modify: `site/src/styles.css:5-18, 31-31`
- Test: `site/src/App.test.tsx`

**Interfaces:**
- Consumes: `DOWNLOAD_URL` from `site/src/download.ts`, which resolves to `/downloads/VoxteraLauncher.exe`.
- Produces: The `<img className="hero-image">` renders `/images/voxtera-hero-reference.png`; its CTAs retain `href={DOWNLOAD_URL}`.

- [ ] **Step 1: Write the failing test**

Add a focused assertion to `site/src/App.test.tsx` that proves the hero loads the reference artwork and keeps the direct launcher path:

```tsx
expect(screen.getByAltText("Paisagem ensolarada de Voxtera com aventureiro e vila")).toHaveAttribute(
  "src",
  "/images/voxtera-hero-reference.png",
);
expect(screen.getAllByRole("link", { name: "Baixar launcher para Windows (.exe)" })[0]).toHaveAttribute(
  "href",
  "/downloads/VoxteraLauncher.exe",
);
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `pnpm test -- App.test.tsx`

Expected: FAIL because the current hero still references `forest-dawn.jpg`.

- [ ] **Step 3: Add the supplied artwork and implement the layout update**

Copy `C:/Users/Gabriel/AppData/Local/Temp/codex-clipboard-4eaf0abe-a2d2-4fcf-bf16-b1dc46dead1e.png` to `site/public/images/voxtera-hero-reference.png`. In `App.tsx`, set the hero image source and alt text to the test values. In `styles.css`, make the hero bright and welcoming: use a warm translucent left-to-right gradient, deep green text, a moss CTA with warm outline, and desktop/mobile `object-position` values that preserve the right-hand adventurer and village.

- [ ] **Step 4: Run verification**

Run: `pnpm test -- App.test.tsx && pnpm build`

Expected: tests pass and Vite produces `dist` without errors.

- [ ] **Step 5: Validate the rendered published flow**

Deploy the site, then verify `https://voxtera.vercel.app/` loads the new hero artwork and `https://voxtera.vercel.app/downloads/VoxteraLauncher.exe` returns the Windows executable.

- [ ] **Step 6: Commit**

```bash
git add site/public/images/voxtera-hero-reference.png site/src/App.tsx site/src/styles.css site/src/App.test.tsx
git commit -m "feat: align Voxtera hero with approved reference"
```

## Self-review

- Spec coverage: Task 1 adds the supplied art, bright left-aligned composition, green CTA treatment, responsive crop, and preserves all direct-download constraints. The non-hero sections are not modified.
- Placeholder scan: no placeholders or deferred implementation steps remain.
- Type consistency: `DOWNLOAD_URL` remains the only launcher URL interface; no new runtime types or APIs are introduced.
