import { describe, expect, it } from 'vitest';
import viteConfig, { WEBVIEW_TARGET } from '../vite.config';

describe('supported launcher platform', () => {
  it('targets the Windows WebView2 baseline without a cross-platform fallback', () => {
    expect(WEBVIEW_TARGET).toBe('chrome105');
    expect(viteConfig).toMatchObject({ build: { target: WEBVIEW_TARGET } });
  });
});
