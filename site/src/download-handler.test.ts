import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const LATEST_RELEASE_URL = "https://api.github.com/repos/Stoltemberg/voxtera/releases/latest";
const RELEASES_URL = "https://github.com/Stoltemberg/voxtera/releases";
type Release = { assets: { name: string; browser_download_url: string }[] };
type Redirect = { statusCode: number; url: string };

function createResponse() {
  const redirects: Redirect[] = [];
  return { redirects, response: { redirect(statusCode: number, url: string) { redirects.push({ statusCode, url }); } } };
}
async function loadHandler() { return (await import("../api/download.js")).default; }

describe("download handler", () => {
  const fetchMock = vi.fn();
  beforeEach(() => { vi.resetModules(); vi.stubGlobal("fetch", fetchMock); });
  afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks(); fetchMock.mockReset(); });

  it("redirects a valid HTTPS launcher asset", async () => {
    fetchMock.mockResolvedValue(new Response(JSON.stringify({ assets: [{ name: "VoxteraLauncher.exe", browser_download_url: "https://github.com/Stoltemberg/voxtera/releases/download/v1/VoxteraLauncher.exe" }] }), { status: 200 }));
    const { redirects, response } = createResponse();
    await (await loadHandler())({}, response);
    expect(redirects).toEqual([{ statusCode: 307, url: "https://github.com/Stoltemberg/voxtera/releases/download/v1/VoxteraLauncher.exe" }]);
    expect(fetchMock).toHaveBeenCalledWith(LATEST_RELEASE_URL, expect.objectContaining({ headers: { "User-Agent": "VoxteraDownloadSite" }, signal: expect.any(AbortSignal) }));
  });

  it.each<Release>([
    { assets: [] },
    { assets: [{ name: "VoxteraLauncher.exe", browser_download_url: "http://example.test/VoxteraLauncher.exe" }] },
  ])("redirects to release history for unavailable or unsafe assets", async (release) => {
    fetchMock.mockResolvedValue(new Response(JSON.stringify(release), { status: 200 }));
    const { redirects, response } = createResponse();
    await (await loadHandler())({}, response);
    expect(redirects).toEqual([{ statusCode: 302, url: RELEASES_URL }]);
  });

  it("redirects to release history when GitHub fails", async () => {
    fetchMock.mockRejectedValue(new Error("GitHub unavailable"));
    const { redirects, response } = createResponse();
    await (await loadHandler())({}, response);
    expect(redirects).toEqual([{ statusCode: 302, url: RELEASES_URL }]);
  });

  it("redirects to release history when GitHub times out", async () => {
    const timeoutController = new AbortController();
    const timeoutSpy = vi.spyOn(AbortSignal, "timeout").mockReturnValue(timeoutController.signal);
    fetchMock.mockImplementation((_url: string, options?: RequestInit) => new Promise((_resolve, reject) => options?.signal?.addEventListener("abort", () => reject(new DOMException("Timed out", "AbortError")))));
    const { redirects, response } = createResponse(); const request = (await loadHandler())({}, response);
    timeoutController.abort(); await expect(request).resolves.toBeUndefined();
    expect(timeoutSpy).toHaveBeenCalledOnce(); expect(redirects).toEqual([{ statusCode: 302, url: RELEASES_URL }]);
  });

  it("uses a cached resolved redirect for subsequent downloads", async () => {
    fetchMock.mockResolvedValue(new Response(JSON.stringify({ assets: [{ name: "VoxteraLauncher.exe", browser_download_url: "https://github.com/Stoltemberg/voxtera/releases/download/v1/VoxteraLauncher.exe" }] }), { status: 200 }));
    const handler = await loadHandler(); const first = createResponse(); const second = createResponse();
    await handler({}, first.response); await handler({}, second.response);
    expect(first.redirects).toEqual([{ statusCode: 307, url: "https://github.com/Stoltemberg/voxtera/releases/download/v1/VoxteraLauncher.exe" }]);
    expect(second.redirects).toEqual(first.redirects); expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
