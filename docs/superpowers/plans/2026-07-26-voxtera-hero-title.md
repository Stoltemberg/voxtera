# Voxtera Hero Title Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Voxtera hero message and browser title with `Sua aventura começa aqui`.

**Architecture:** This is a copy-only change. The React component owns the visible hero heading, while `index.html` owns the static document title. A focused component assertion protects the visible copy; the production build validates the static HTML change.

**Tech Stack:** React, TypeScript, Vite, Vitest.

## Global Constraints

- Use the exact visible copy `Sua aventura começa aqui`.
- Set the document title to `Voxtera — Sua aventura começa aqui`.
- Preserve the existing hero visual treatment and all launcher links at `/downloads/VoxteraLauncher.exe`.
- Add no GitHub link or external download redirect.

---

### Task 1: Update and verify the hero copy

**Files:**
- Modify: `site/src/App.test.tsx`
- Modify: `site/src/App.tsx:25`
- Modify: `site/index.html:7`

**Interfaces:**
- Consumes: the existing semantic `<h1 id="hero-title">` and direct `DOWNLOAD_URL` contract.
- Produces: a hero heading exposed to assistive technologies as `Sua aventura começa aqui` and a matching HTML document title.

- [ ] **Step 1: Write the failing heading assertion**

Add this test to `site/src/App.test.tsx`:

```tsx
it("uses the approved hero adventure message", () => {
  render(<App />);

  expect(
    screen.getByRole("heading", { level: 1, name: "Sua aventura começa aqui" }),
  ).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run:

```powershell
pnpm test -- App.test.tsx
```

Expected: FAIL because the current hero heading is `Seu mundo voxel começa aqui`.

- [ ] **Step 3: Implement the approved wording**

In `site/src/App.tsx`, replace the heading with:

```tsx
<h1 id="hero-title">Sua aventura começa aqui</h1>
```

In `site/index.html`, replace the document title with:

```html
<title>Voxtera — Sua aventura começa aqui</title>
```

- [ ] **Step 4: Run the focused test and production build**

Run:

```powershell
pnpm test -- App.test.tsx
pnpm build
```

Expected: the page test passes and Vite produces `dist/` without TypeScript errors.

- [ ] **Step 5: Commit the verified copy update**

```powershell
git add site/src/App.tsx site/src/App.test.tsx site/index.html
git commit -m "fix: update Voxtera hero message"
```

## Plan self-review

- **Spec coverage:** Task 1 changes both approved copy locations and keeps the direct launcher contract untouched.
- **Placeholder scan:** No deferred work or placeholder language remains.
- **Interface consistency:** The existing `hero-title` identifier and `DOWNLOAD_URL` remain unchanged; the test targets the final accessible heading text.
