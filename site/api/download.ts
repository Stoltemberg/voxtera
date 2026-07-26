import { findLauncherUrl, isLauncherDownloadUrl } from "../src/release.js";

const LATEST_RELEASE_URL = "https://api.github.com/repos/Stoltemberg/voxtera/releases/latest";
const RELEASES_URL = "https://github.com/Stoltemberg/voxtera/releases";
const FETCH_TIMEOUT_MS = 3_000;
const REDIRECT_CACHE_TTL_MS = 5 * 60 * 1_000;

type Redirect = { statusCode: 302 | 307; url: string };

let cachedRedirect: { redirect: Redirect; expiresAt: number } | undefined;
let resolvingRedirect: Promise<Redirect> | undefined;

function fallbackRedirect(): Redirect {
  return { statusCode: 302, url: RELEASES_URL };
}

async function resolveRedirect(): Promise<Redirect> {
  const now = Date.now();
  if (cachedRedirect && cachedRedirect.expiresAt > now) {
    return cachedRedirect.redirect;
  }

  if (!resolvingRedirect) {
    resolvingRedirect = (async () => {
      try {
        const latestRelease = await fetch(LATEST_RELEASE_URL, {
          headers: { "User-Agent": "VoxteraDownloadSite" },
          signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
        });

        if (!latestRelease.ok) {
          return fallbackRedirect();
        }

        const launcherUrl = findLauncherUrl(await latestRelease.json());
        if (typeof launcherUrl !== "string" || !isLauncherDownloadUrl(launcherUrl)) {
          return fallbackRedirect();
        }

        return { statusCode: 307, url: launcherUrl };
      } catch {
        return fallbackRedirect();
      }
    })();
  }

  try {
    const redirect = await resolvingRedirect;
    cachedRedirect = { redirect, expiresAt: Date.now() + REDIRECT_CACHE_TTL_MS };
    return redirect;
  } finally {
    resolvingRedirect = undefined;
  }
}

export default async function handler(
  _request: unknown,
  response: { redirect: (statusCode: number, url: string) => void },
) {
  const redirect = await resolveRedirect();
  return response.redirect(redirect.statusCode, redirect.url);
}
