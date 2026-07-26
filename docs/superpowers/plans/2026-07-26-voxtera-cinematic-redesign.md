# Voxtera Cinematic Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the Voxtera landing page into a cinematic, image-led presentation without changing its direct executable download.

**Architecture:** Keep the existing React page and its public image assets, but replace the card/grid composition with a cinematic hero, broad editorial image bands, a typography-led start sequence, and a centered final CTA. Styling remains in the shared stylesheet; no new runtime dependencies are required.

**Tech Stack:** React, TypeScript, CSS, Vite, Vitest, Vercel.

## Global Constraints

- Reuse only assets already under `site/public/images/`.
- Every launcher CTA must retain `/downloads/VoxteraLauncher.exe`.
- The published page must not contain GitHub links or external download redirects.
- Keep `voxtera.vercel.app` as the public domain.

---

### Task 1: Replace the card-driven composition with cinematic sections

**Files:**
- Modify: `site/src/App.tsx`
- Modify: `site/src/styles.css`
- Modify: `site/src/App.test.tsx`

**Interfaces:**
- Consumes: `DOWNLOAD_URL` from `site/src/download.ts` and existing `/images/*.jpg|png` assets.
- Produces: one panoramic hero, editorial wide-media sections, numbered start sequence, and centered final CTA.

- [ ] **Step 1: Write the failing structure test**

Add a test that asserts the rendered document contains no `feature-card`, no `step-illustration`, and contains the two wide-media section labels `Explore` and `Aventure-se`.

- [ ] **Step 2: Run the test and observe it fail**

Run: `pnpm test -- App.test.tsx`

Expected: FAIL because the old card and thumbnail composition still renders.

- [ ] **Step 3: Implement the cinematic page structure**

Use one full-bleed `forest-dawn.jpg` hero. Replace square feature rows with two broad editorial sections using `mountain-valley.jpg` and `ruins-adventure.jpg`; one section has copy over the lower edge of its wide image and the other pairs a wide image with open copy. Render the start sequence as three numbered text columns without image tags. Center the final title, supporting line, and `DOWNLOAD_URL` button over the `ruins-adventure.jpg` band. Keep the footer compact and wood-toned.

- [ ] **Step 4: Implement responsive media treatment**

Use stable wide aspect ratios, `object-fit: cover`, and mobile-specific crop positions. At widths below 760px, stack editorial text and media, retain full-width CTA buttons, and avoid fixed-height square image panels.

- [ ] **Step 5: Run checks and commit**

Run:

```powershell
pnpm test
pnpm build
git add site/src/App.tsx site/src/styles.css site/src/App.test.tsx
git commit -m "feat: redesign Voxtera landing page cinematically"
```

Expected: all tests pass and Vite builds successfully.

### Task 2: Render QA and publish the cinematic redesign

**Files:**
- Modify: `site/README.md` only if the deployment URL changes.

**Interfaces:**
- Consumes: production build from Task 1 and `voxtera.vercel.app`.
- Produces: visually verified responsive deployment.

- [ ] **Step 1: Start the local server and capture desktop/mobile screenshots**

Run: `pnpm dev -- --host 127.0.0.1`

Inspect desktop (1440px) and mobile (390px) pages. Confirm the hero is one coherent scene, imagery is wide rather than square, the direct executable button is visible, and no GitHub link appears.

- [ ] **Step 2: Publish production**

Run: `pnpm dlx vercel --prod --yes`

Expected: deployment reaches `READY` and retains the `voxtera.vercel.app` alias.

- [ ] **Step 3: Verify public page and direct executable**

Run:

```powershell
curl.exe -sS https://voxtera.vercel.app | Select-String -Pattern 'github.com' -Quiet
curl.exe -sS -D - -o NUL https://voxtera.vercel.app/downloads/VoxteraLauncher.exe
```

Expected: the HTML check returns `False`; the executable response returns `200 OK`.

## Plan self-review

- **Spec coverage:** Task 1 applies each approved composition and media constraint; Task 2 verifies responsive rendering, public domain, direct download, and absence of GitHub links.
- **Placeholder scan:** No deferred implementation items remain.
- **Interface consistency:** The existing `DOWNLOAD_URL` remains the single download target throughout both tasks.
