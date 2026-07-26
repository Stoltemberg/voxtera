# Voxtera download site

An independently buildable Vite and React site for the Voxtera download experience.

## Commands

Run these from the `site/` directory:

- `npm install` installs the site dependencies.
- `npm run dev -- --host 127.0.0.1` starts the local development server at the URL printed by Vite (normally `http://127.0.0.1:5173`).
- `npm run build` type-checks the project and creates a production build in `dist/`.
- `npm test` runs the Vitest suite.

The Windows launcher executable is served at `/downloads/VoxteraLauncher.exe`.

Project-owned image copies are stored in `public/images/`.

## Production

The live site is available at https://voxtera.vercel.app.
