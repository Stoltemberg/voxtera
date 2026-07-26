# Voxtera cinematic redesign

## Goal

Replace the amateur-looking square image-card treatment with an image-led,
cinematic landing page while retaining the direct Windows launcher download.

## Visual system

- The hero is one panoramic gameplay scene with a restrained text-legibility
  gradient and no framed image card.
- The page uses open editorial layouts: short text blocks alternate with
  full-bleed or wide 16:9/21:9 gameplay frames.
- The three gameplay themes use typography and broad scenes rather than
  square thumbnails or card grids.
- The start sequence is a simple horizontal numbered line without image
  thumbnails.
- The closing download section is a dark full-width scene with centered copy
  and a single direct-download CTA.
- The wood footer remains but is simplified to avoid visual clutter.

## Constraints

- Reuse only project-owned game imagery already in `site/public/images/`.
- Keep every launcher CTA pointing at `/downloads/VoxteraLauncher.exe`.
- Do not add any GitHub link or external download redirect.
- Preserve responsive behavior and the existing public `voxtera.vercel.app`
  deployment.

## Acceptance criteria

- No square gameplay-card grid or thumbnail-driven step sequence remains.
- The first viewport reads as one coherent game scene.
- Desktop and mobile keep readable text, unclipped media, and a visible
  direct `.exe` download action.
