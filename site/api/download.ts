import { findLauncherUrl, isLauncherDownloadUrl } from "../src/release";

const LATEST_RELEASE_URL = "https://api.github.com/repos/Stoltemberg/voxtera/releases/latest";
const RELEASES_URL = "https://github.com/Stoltemberg/voxtera/releases";

export default async function handler(
  _request: unknown,
  response: { redirect: (statusCode: number, url: string) => void },
) {
  try {
    const latestRelease = await fetch(LATEST_RELEASE_URL, {
      headers: { "User-Agent": "VoxteraDownloadSite" },
    });

    if (!latestRelease.ok) {
      return response.redirect(302, RELEASES_URL);
    }

    const launcherUrl = findLauncherUrl(await latestRelease.json());
    if (typeof launcherUrl !== "string" || !isLauncherDownloadUrl(launcherUrl)) {
      return response.redirect(302, RELEASES_URL);
    }

    return response.redirect(307, launcherUrl);
  } catch {
    return response.redirect(302, RELEASES_URL);
  }
}
