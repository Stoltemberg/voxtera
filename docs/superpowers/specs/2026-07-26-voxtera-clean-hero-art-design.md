# Voxtera clean hero artwork

## Goal

Replace the current reference screenshot used as a hero background with a
clean, interface-free Voxtera landscape that keeps the game world visibly
present across the hero.

## Visual design

- Create one panoramic voxel-game artwork inspired by the approved reference:
  a bright open valley, village, river, distant mountains and an adventurer
  with a companion on the right.
- The generated artwork must contain no logo, words, interface, buttons,
  navigation or watermark.
- Use the artwork as the full hero background. The left text area receives
  only a restrained warm legibility gradient; the center and right must keep
  the scene clear and colorful.
- Preserve the current live wordmark, navigation, direct `.exe` CTA, hero
  copy and all sections below the hero.

## Responsive behavior

- Desktop shows the wide valley and keeps the adventurer/village on the right.
- Mobile uses a right-biased crop that preserves the adventurer and nearby
  village, while a shorter left-side gradient keeps copy readable.

## Constraints

- Every launcher CTA continues to point to `/downloads/VoxteraLauncher.exe`.
- Add no GitHub link or external download redirect.
- Use the generated image only as a local public asset.

## Acceptance criteria

- No baked-in interface elements appear in the hero image.
- The hero's background remains materially visible behind and to the right of
  live content at desktop and mobile sizes.
- Existing headline, navigation and direct download controls remain legible
  and functional.
