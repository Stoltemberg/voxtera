# Voxtera editorial section refresh

## Goal

Give the Voxtera launcher page more life below the hero by translating the
approved voxel-editorial references into a cohesive build, onboarding and
closing-download sequence.

## Scope and invariants

- Preserve the existing hero artwork, header, headline `Sua aventura começa
  aqui`, and every direct launcher link at `/downloads/VoxteraLauncher.exe`.
- Preserve the light parchment background, forest-green serif headings,
  warm-gold accents and image-led voxel world already established by the page.
- Add no GitHub link, external download redirect, new navigation item, card
  grid, pill, metric, or above-the-fold visible copy.
- Keep all interface text, headings, buttons and step descriptions as
  code-native HTML; generated artwork must contain no interface or text.

## Section design

### Build feature: `Construa`

- Insert a calm editorial build feature after the existing exploration moment
  and before the current adventure moment.
- On desktop, use an open two-column composition: dark-moss crossed
  hammer/pickaxe icon, `Construa` heading, warm-gold ornamental divider and
  short copy on the left; one generous rounded 16:9 voxel village image on
  the right.
- The image shows handmade timber houses, gardens, paths and distant
  mountains under warm daylight. It contains no people, UI or text.
- On mobile, stack the copy above the image with the same side gutters and a
  stable landscape crop.

### Onboarding: `Como começar`

- Replace the current plain step-list treatment with a more ceremonial,
  centered onboarding section titled `Como começar`, framed by small warm-gold
  line-and-diamond ornaments.
- Keep exactly three steps and the existing launcher flow. Use the headings
  `Baixe o launcher`, `Instale o jogo`, and `Entre em Voxtera`.
- Each step receives one bespoke voxel prop: a wooden launcher chest, a
  glowing green stone portal and a sword-with-shield. Desktop steps are
  connected by a fine dotted gold path with small green numbered medallions.
- Props are local transparent PNG assets on a chroma-key-removal workflow;
  they are visually rich but do not contain labels, numerals or text.
- On mobile, the path becomes a vertical guide and the three steps retain a
  clear reading order and generous tap-safe spacing.

### Closing download band

- Replace the existing dark ruins treatment with a dedicated full-width
  voxel valley: an adventurer frames the lower left, a gray wolf frames the
  lower right, and trees/flowers create a natural edge frame.
- Preserve an unbusy, deep-green center with only a subtle native CSS shade
  behind the live heading, explanatory text and direct `.exe` CTA. The scene
  itself has no baked-in UI or wording.
- Use the heading `Pronto para começar sua aventura?`; keep the existing
  direct download label exactly `Baixar launcher para Windows (.exe)`.
- On mobile, preserve the CTA and one character/wolf crop without hiding the
  central copy.

## Asset plan

- Generate five local image assets from the approved concepts: one village
  editorial landscape, three isolated onboarding props, and one wide closing
  valley. No approved concept screenshot is used as a production asset.
- Store final landscapes under `site/public/images/`; store local transparent
  PNG props in the same directory. Do not replace existing image files.
- Use a dark-moss custom SVG for the crossed hammer/pickaxe section icon so
  it remains sharp and responsive.

## Visual system

- Background: the current warm parchment (`--paper`), not pure white.
- Typography: existing Georgia-style display serif for headings; existing
  sans-serif for body and controls.
- Accent: existing moss green and warm gold, with one-pixel gold dividers,
  dotted paths and restrained gold outlines only where the approved concept
  shows them.
- Media: village image with a 16px radius; onboarding props float without
  card containers; closing landscape is full bleed with a localized center
  shade only.

## Acceptance criteria

- The page has visibly distinct `Construa`, `Como começar` and closing
  download moments matching the approved concepts.
- The new imagery and props load as local site assets, contain no baked UI,
  and remain readable at desktop and mobile sizes.
- The visible hero and direct `.exe` download behavior remain unchanged.
- Tests cover the new visible section headings, all direct CTA destinations,
  and the generated asset paths. A production build succeeds.
