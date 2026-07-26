# Voxtera hero reference alignment

## Goal

Replace the current forest hero with the user-provided Voxtera artwork and
match the supplied reference's bright, welcoming composition.

## Visual design

- Add the user-provided PNG to the site-owned public image assets and use it
  as the hero background.
- Keep the hero as a single full-bleed panoramic scene. Preserve the character
  and village view on the right side of the composition.
- Use a soft, light left-side legibility gradient so the heading, supporting
  copy and platform note read in deep green rather than white.
- Make the wordmark large at desktop width, retain the existing navigation
  labels and direct `.exe` download link, and style the header button in the
  reference's moss green with a restrained warm outline.
- Keep the hero copy and direct launcher CTA on the left. The CTA remains
  `/downloads/VoxteraLauncher.exe`; no external or GitHub link is added.
- Retain the cinematic sections below the fold unchanged.

## Responsive behavior

- Desktop uses the supplied wide composition with the character visible on the
  right and the text block clear on the left.
- Mobile prioritizes readable left-aligned copy and the character/village area
  through a mobile-specific background position and a stronger light gradient.

## Acceptance criteria

- The top of `voxtera.vercel.app` uses the supplied artwork rather than
  `forest-dawn.jpg`.
- The first viewport visually matches the user-supplied bright, welcoming
  reference: dark green type on the left and the game world unobstructed to
  the right.
- Every download CTA still points directly to the Windows `.exe` asset.
- The existing gameplay sections below the hero are unchanged.
