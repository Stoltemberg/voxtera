# Voxtera download site design

## Goal

Create and deploy a polished, welcoming landing page that distributes the
Windows Voxtera launcher executable. The page must present the game as a
voxel adventure, reuse suitable project-owned game imagery, and make the
download action unmistakable.

## Visual direction

**Adventure at dawn.** The page uses a bright sky, green vegetation, warm
wood details, and a subtle paper-like neutral surface. A full-width game
capture anchors the hero. A restrained dark-to-transparent gradient preserves
headline legibility without obscuring the scene.

The interface will use a display serif for headings with a clear sans-serif
for body and UI text. The accent color is a forest green used for download
buttons and small interactive states. Content remains spacious, friendly, and
easy to scan on desktop and mobile.

## Page structure

1. **Header** — Voxtera wordmark, links to `O jogo` and `Como começar`, plus
   a compact download control.
2. **Hero** — game image, the message `Seu mundo voxel começa aqui`, short
   supporting copy, and the primary `Baixar launcher para Windows (.exe)` CTA.
   Supporting text states `Windows 10/11 · instalação e atualizações
   automáticas`.
3. **Experience** — an image-led narrative with the three themes `Explore`,
   `Construa`, and `Aventure-se`.
4. **Getting started** — three concise steps: download the launcher, install
   it, then play.
5. **Closing download band** — repeats the executable download CTA for people
   who scroll through the page before deciding.
6. **Footer** — links to the GitHub repository and release history.

## Download behavior

The primary and repeated CTAs will resolve the current GitHub release's
`VoxteraLauncher.exe` asset rather than pinning a version number in the page.
The deployed site may implement that resolution through a lightweight Vercel
route that redirects to the matching release asset. If the asset is absent,
the route will redirect visitors to the releases page, which is a safe,
actionable fallback.

## Assets

The implementation will prefer suitable images and marks already in this
repository. It will not depend on temporary generated imagery. Any copied
web assets will live inside the website project so that the Vercel deployment
is self-contained.

## Implementation and deployment

The site will be an isolated Vite/React project under `site/`, with responsive
semantic markup and no runtime configuration required for the static landing
page. A minimal Vercel function may be added only for the latest-release
redirect.

It will be validated through a production build and browser checks at desktop
and mobile sizes before deploying to Vercel production. The final handoff
will include the public deployment URL.

## Launcher cleanup

The obsolete Tauri launcher worktree at `.worktrees/launcher-site` will be
removed after the site is implemented and verified. This cleanup must not
modify the retained Python launcher in `launcher/`, including
`launcher/voxtera_launcher.py` and its packaged executable.

## Acceptance criteria

- A clear, attractive and responsive Voxtera distribution landing page exists.
- The primary CTA downloads or resolves to the latest Windows launcher `.exe`.
- The page uses project-owned Voxtera/game imagery.
- The Tauri worktree is removed while the Python launcher remains intact.
- The production Vercel deployment succeeds and its URL is provided.
