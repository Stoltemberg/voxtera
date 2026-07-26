# Voxtera download site

An independently buildable Vite and React site for the Voxtera download experience.

## Commands

Run these from the `site/` directory:

- `npm install` installs the site dependencies.
- `npm run dev -- --host 127.0.0.1` starts the local development server at the URL printed by Vite (normally `http://127.0.0.1:5173`).
- `npm run build` type-checks the project and creates a production build in `dist/`.
- `npm test` runs the Vitest suite.

The desktop download page is available at `/download`; the primary homepage CTA links there.

Project-owned image copies are stored in `public/images/`.

## Production

The live site is available at https://site-gilt-psi-44.vercel.app.

The `/download` route resolves the current `VoxteraLauncher.exe` from GitHub
releases. When no matching executable is published, it safely redirects to the
release history page instead.
