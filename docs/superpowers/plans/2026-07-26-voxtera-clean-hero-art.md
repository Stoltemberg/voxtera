# Voxtera Clean Hero Artwork Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the washed-out reference screenshot with a clean, fully visible voxel landscape hero that retains Voxtera's direct launcher download.

**Architecture:** A generated, interface-free landscape is stored as a local public image and consumed by the existing `.hero-image`. The existing React structure and launcher URL stay unchanged; CSS narrows the legibility gradient so only the live text area is protected while the center and right preserve the generated scene.

**Tech Stack:** React, TypeScript, Vite, CSS, Vitest, image generation, Vercel.

## Global Constraints

- The hero image contains no logo, words, interface, buttons, navigation or watermark.
- Every launcher CTA continues to point to `/downloads/VoxteraLauncher.exe`.
- Add no GitHub link or external download redirect.
- Use the generated image only as a local public asset.
- Preserve all content below the hero.
- Desktop and mobile keep live text readable while visibly retaining the scene and right-hand adventurer/village.

---

### Task 1: Generate and integrate clean hero art

**Files:**
- Create: `site/public/images/voxtera-clean-hero.png`
- Modify: `site/src/App.tsx:14`
- Modify: `site/src/styles.css:4-14`
- Modify: `site/src/App.test.tsx:31-40`

**Interfaces:**
- Consumes: `DOWNLOAD_URL` from `site/src/download.ts`, which is `/downloads/VoxteraLauncher.exe`.
- Produces: `.hero-image` loads `/images/voxtera-clean-hero.png`; the live CTA URLs remain unchanged.

- [ ] **Step 1: Generate the artwork**

Generate one 16:9 panoramic voxel RPG landscape with a sunlit open valley, river, cozy village, distant stone mountains and an adventurer plus small companion on the right foreground. The image must have no text, logo, user interface, buttons, watermark or border. Save the accepted result as `site/public/images/voxtera-clean-hero.png`.

- [ ] **Step 2: Write the failing test**

Update the existing hero-art assertion in `site/src/App.test.tsx` before changing the application:

```tsx
expect(screen.getByAltText("Vale ensolarado de Voxtera com aventureiro e vila")).toHaveAttribute(
  "src",
  "/images/voxtera-clean-hero.png",
);
```

- [ ] **Step 3: Run the focused test to verify RED**

Run: `pnpm test -- App.test.tsx`

Expected: FAIL because `App.tsx` still points at `/images/voxtera-hero-reference.png`.

- [ ] **Step 4: Integrate the generated art and narrow the overlay**

Update the hero image `src` and alt text to the values in Step 2. Replace the opaque full-width/top-band masking with a restrained warm left-side gradient that ends by the center of the hero. Keep the live header and hero content above the image. On mobile, use a right-biased `object-position` so the adventurer/village remain in frame while the left gradient protects readable text.

- [ ] **Step 5: Verify GREEN and production build**

Run: `pnpm test -- App.test.tsx && pnpm build`

Expected: all four tests pass and Vite creates `dist` without errors.

- [ ] **Step 6: Publish and visually verify**

Deploy to Vercel production and assign `voxtera.vercel.app`. Verify desktop and a 390px mobile viewport: hero scene is materially visible, no baked interface exists, no console errors occur, and `/downloads/VoxteraLauncher.exe` returns HTTP 200.

- [ ] **Step 7: Commit**

```bash
git add site/public/images/voxtera-clean-hero.png site/src/App.tsx site/src/styles.css site/src/App.test.tsx
git commit -m "feat: restore visible Voxtera hero scenery"
```

## Self-review

- Spec coverage: Task 1 generates an interface-free local asset, makes it the hero background, narrows the overlay, protects responsive readability and preserves all direct-download constraints.
- Placeholder scan: no deferred or ambiguous implementation instruction remains.
- Type consistency: `DOWNLOAD_URL` is unchanged; the task only changes the static image path and CSS presentation.
