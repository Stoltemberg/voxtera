import { describe, expect, it } from "vitest";
import { findLauncherUrl, isLauncherDownloadUrl } from "./release";

describe("findLauncherUrl", () => {
  it("returns the exact Windows launcher executable", () => {
    expect(findLauncherUrl({
      assets: [
        { name: "game.zip", browser_download_url: "https://example.test/game.zip" },
        { name: "VoxteraLauncher.exe", browser_download_url: "https://example.test/launcher.exe" }
      ]
    })).toBe("https://example.test/launcher.exe");
  });

  it("returns null when the executable is absent", () => {
    expect(findLauncherUrl({ assets: [] })).toBeNull();
  });

  it("rejects a malformed launcher download URL", () => {
    expect(isLauncherDownloadUrl("not a URL")).toBe(false);
  });
});
