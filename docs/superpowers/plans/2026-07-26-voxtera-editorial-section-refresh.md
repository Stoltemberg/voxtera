# Voxtera Editorial Section Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an editorial `Construa` feature, an object-led `Como começar` flow and a richer final download scene to the Voxtera page.

**Architecture:** `App.tsx` remains the page-composition owner and receives a small local `BuildIcon` SVG plus data-backed onboarding props. `styles.css` extends the established parchment, moss and warm-gold system with three section-specific layouts. Five generated local images live under `site/public/images/` and are referenced by static URLs.

**Tech Stack:** React 19, TypeScript, Vite, CSS, Vitest, React Testing Library, built-in Image Gen.

## Global Constraints

- Preserve the existing hero artwork, header, headline `Sua aventura começa aqui`, and every direct launcher link at `/downloads/VoxteraLauncher.exe`.
- Keep all interface text, headings, buttons and step descriptions as code-native HTML; generated artwork must contain no interface or text.
- Add no GitHub link, external download redirect, new navigation item, card grid, pill, metric, or above-the-fold visible copy.
- Use the exact section labels `Construa`, `Como começar`, `Baixe o launcher`, `Instale o jogo`, and `Entre em Voxtera`.
- Store five new non-destructive local assets under `site/public/images/`; do not overwrite existing images.
- Preserve readable desktop and mobile layouts, including the direct `.exe` CTA.

---

## Planned file structure

- `site/public/images/voxtera-build-village.png` — wide voxel village landscape for `Construa`.
- `site/public/images/voxtera-step-chest.png` — isolated voxel chest prop with alpha transparency.
- `site/public/images/voxtera-step-portal.png` — isolated voxel portal prop with alpha transparency.
- `site/public/images/voxtera-step-sword-shield.png` — isolated voxel sword-and-shield prop with alpha transparency.
- `site/public/images/voxtera-closing-valley.png` — wide closing valley with an adventurer and gray wolf at the outer edges.
- `site/src/App.tsx` — semantic sections, custom build SVG, prop metadata and direct CTA composition.
- `site/src/App.test.tsx` — visible copy, local image and direct-download regression tests.
- `site/src/styles.css` — responsive editorial build, onboarding and closing-band treatments.

### Task 1: Add the approved local imagery and semantic page structure

**Files:**
- Create: the five assets in Planned file structure
- Modify: `site/src/App.tsx`
- Modify: `site/src/App.test.tsx`

**Interfaces:**
- Consumes: `DOWNLOAD_URL` from `site/src/download.ts`, which must remain `/downloads/VoxteraLauncher.exe`.
- Produces: an editorial `Construa` article, a `Como começar` onboarding section and five image elements using the exact asset paths.

- [ ] **Step 1: Write a failing semantic and asset test**

Add this test to `site/src/App.test.tsx`:

```tsx
it("renders the approved build and onboarding visual language", () => {
  render(<App />);

  expect(screen.getByRole("heading", { name: "Construa" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Como começar" })).toBeInTheDocument();
  expect(screen.getByAltText("Vila voxel ensolarada para construir em Voxtera")).toHaveAttribute("src", "/images/voxtera-build-village.png");
  expect(screen.getByAltText("Baú voxel do launcher")).toHaveAttribute("src", "/images/voxtera-step-chest.png");
  expect(screen.getByAltText("Portal voxel para instalar o jogo")).toHaveAttribute("src", "/images/voxtera-step-portal.png");
  expect(screen.getByAltText("Espada e escudo voxel para entrar em Voxtera")).toHaveAttribute("src", "/images/voxtera-step-sword-shield.png");
  expect(screen.getByAltText("Vale voxel com aventureiro e lobo")).toHaveAttribute("src", "/images/voxtera-closing-valley.png");
});
```

- [ ] **Step 2: Run the focused test to prove the current page fails**

Run:

```powershell
pnpm test -- App.test.tsx
```

Expected: FAIL because the headings and the five asset paths are absent.

- [ ] **Step 3: Generate and copy five interface-free local assets**

Use built-in Image Gen once per prompt, then copy selected results into the five exact paths. For the props, generate on a flat `#ff00ff` chroma-key background, remove it with `C:/Users/Gabriel/.codex/skills/.system/imagegen/scripts/remove_chroma_key.py`, and validate transparent corners.

```text
1. Wide voxel fantasy village, handmade timber houses, gardens, stone paths, distant mountains and warm daylight; no characters, UI, words, logos or watermark; no rounded frame; 16:9 landscape.
2. Single handcrafted wooden voxel launcher chest with metal corners and a small green gem lock, centered on flat #ff00ff; no text, numerals, shadow, watermark or other objects; generous padding.
3. Single weathered gray stone voxel portal glowing soft green, centered on flat #ff00ff; no text, numerals, shadow, watermark or other objects; generous padding.
4. Single voxel sword crossed in front of a moss-green wooden shield, centered on flat #ff00ff; no text, numerals, shadow, watermark or other objects; generous padding.
5. Wide deep-green voxel valley: adventurous character at far lower left, friendly gray wolf at far lower right, trees and flowers framing the outer edges, clear dark empty center for live text; no UI, words, logos or watermark; 16:9 landscape.
```

- [ ] **Step 4: Compose the exact new semantic content**

Add a `BuildIcon` component returning a crossed hammer/pickaxe SVG with `viewBox="0 0 48 48"`, `fill="none"`, `stroke="currentColor"`, `strokeWidth="4"`, `strokeLinecap="round"` and `strokeLinejoin="round"`. Insert between exploration and adventure:

```tsx
<article className="editorial editorial-build page-width" aria-labelledby="build-title">
  <div className="editorial-build-copy">
    <BuildIcon />
    <h3 id="build-title">Construa</h3>
    <div className="ornament" aria-hidden="true" />
    <p>Erga cidades, fortalezas e fazendas. Use blocos, recursos e sua criatividade para transformar o mundo do seu jeito.</p>
  </div>
  <img src="/images/voxtera-build-village.png" alt="Vila voxel ensolarada para construir em Voxtera" />
</article>
```

Change the onboarding heading to `<h2 id="start-title">Como começar</h2>`. Replace `startSteps` tuples with objects carrying `number`, `title`, `text`, `image`, and `alt`; use the specified step assets and alt values from Step 1. Render each prop before its `h3` and a decorative `<span className="steps-path" aria-hidden="true" />` between desktop steps. Replace the closing image with `/images/voxtera-closing-valley.png` and alt `Vale voxel com aventureiro e lobo`. Change the final heading to `Pronto para começar sua aventura?`; leave the direct download anchor unchanged.

- [ ] **Step 5: Run the focused test to prove the structure passes**

Run:

```powershell
pnpm test -- App.test.tsx
```

Expected: PASS with existing direct-download/no-GitHub coverage plus the new assertion.

- [ ] **Step 6: Commit the local assets and semantic structure**

```powershell
git add site/public/images/voxtera-build-village.png site/public/images/voxtera-step-chest.png site/public/images/voxtera-step-portal.png site/public/images/voxtera-step-sword-shield.png site/public/images/voxtera-closing-valley.png site/src/App.tsx site/src/App.test.tsx
git commit -m "feat: add Voxtera editorial game sections"
```

### Task 2: Implement editorial layouts and responsive visual QA

**Files:**
- Modify: `site/src/styles.css`

**Interfaces:**
- Consumes: the `.editorial-build`, `.editorial-build-copy`, `.ornament`, `.step-art`, `.steps-path` and `.download-band` elements from Task 1.
- Produces: a two-column build feature, connected three-step onboarding, a full-bleed character-and-wolf closing band, and mobile layouts without horizontal overflow.

- [ ] **Step 1: Capture the pre-style visual baseline**

Run `pnpm dev -- --host 127.0.0.1`, capture the build/onboarding/closing region at 1440px, and record the missing two-column build spacing, dotted onboarding path and edge-framed closing composition.

- [ ] **Step 2: Add the approved responsive CSS**

Add these core rules and matching responsive overrides:

```css
.editorial-build { align-items:center; display:grid; gap:clamp(40px,8vw,116px); grid-template-columns:minmax(240px,.72fr) minmax(0,1.28fr); padding-block:clamp(94px,11vw,150px); }
.editorial-build-copy svg { color:var(--moss); height:48px; margin-bottom:22px; width:48px; }
.editorial-build-copy h3 { color:var(--moss-deep); }
.ornament { background:linear-gradient(90deg,var(--wood-light) 0 46%,transparent 46% 54%,var(--wood-light) 54%); height:1px; margin:27px 0; max-width:290px; position:relative; }
.ornament::after { border:2px solid var(--wood-light); content:""; height:8px; left:50%; position:absolute; top:50%; transform:translate(-50%,-50%) rotate(45deg); width:8px; }
.editorial-build img { aspect-ratio:16 / 9; border-radius:16px; object-fit:cover; width:100%; }
.steps { column-gap:28px; position:relative; text-align:center; }
.steps li { border:0; padding-top:0; position:relative; z-index:1; }
.step-art { height:clamp(125px,15vw,186px); margin:0 auto 20px; object-fit:contain; width:min(100%,210px); }
.step-number { align-items:center; background:var(--moss); border:2px solid #e1ba62; border-radius:50%; color:#fff; display:flex; height:48px; justify-content:center; margin:0 auto 14px; width:48px; }
.steps-path { border-top:3px dotted #cfab5e; left:17%; position:absolute; right:17%; top:24px; }
.download-band > img { filter:none; object-position:center; }
.download-band-shade { background:radial-gradient(ellipse at center,rgba(11,45,29,.94) 0 28%,rgba(11,45,29,.72) 47%,rgba(11,45,29,.08) 76%); }
```

At `max-width:760px`, stack `.editorial-build` copy before image; make `.steps` one column; hide `.steps-path`; reduce `.step-art` to 132px; keep the closing band at least 500px high and preserve a readable central CTA.

- [ ] **Step 3: Run regression tests and production build**

Run:

```powershell
pnpm test
pnpm build
```

Expected: all tests pass and Vite creates `dist/` without TypeScript errors.

- [ ] **Step 4: Perform desktop and mobile concept-fidelity QA**

Use the in-app browser to capture at desktop width 1440px and mobile width 390px. Inspect accepted concepts and screenshots with `view_image` in one QA pass. Record: (1) parchment/moss/gold palette, (2) build copy/image balance, (3) village radius and crop, (4) prop scale and dotted path, (5) CTA legibility against the closing valley, and (6) mobile stacking/no horizontal overflow. Correct all material mismatches before committing.

- [ ] **Step 5: Commit the visual refresh**

```powershell
git add site/src/styles.css
git commit -m "feat: style Voxtera editorial game journey"
```

## Plan self-review

- **Spec coverage:** Task 1 adds all five local images, exact section copy, semantic structure and direct-download preservation; Task 2 implements desktop/mobile layouts and validates every approved visual requirement.
- **Placeholder scan:** No deferred work, ambiguous asset name or unspecified command remains.
- **Interface consistency:** Task 1 creates every class and asset path consumed by Task 2; the direct `DOWNLOAD_URL` contract remains unchanged.
