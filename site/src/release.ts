type ReleaseAsset = { name: string; browser_download_url: string };
type LatestRelease = { assets?: ReleaseAsset[] };

export function findLauncherUrl(release: LatestRelease): string | null {
  return release.assets?.find((asset) => asset.name === "VoxteraLauncher.exe")
    ?.browser_download_url ?? null;
}
