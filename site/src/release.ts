type ReleaseAsset = { name: string; browser_download_url: string };
type LatestRelease = { assets?: ReleaseAsset[] };

export function findLauncherUrl(release: LatestRelease): string | null {
  return release.assets?.find((asset) => asset.name === "VoxteraLauncher.exe")
    ?.browser_download_url ?? null;
}

export function isLauncherDownloadUrl(value: string): boolean {
  try {
    return new URL(value).protocol === "https:";
  } catch {
    return false;
  }
}
