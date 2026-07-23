import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import brandTokens from '../../brand/tokens.json';

/* ---------- Mock @tauri-apps/api (hoisted) ---------- */

const { mockInvoke, mockListen } = vi.hoisted(() => {
  return {
    mockInvoke: vi.fn(),
    mockListen: vi.fn(),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
}));

/* ---------- Import AFTER mocks ---------- */

import { App } from './App';

/* ---------- Helpers ---------- */

function makeSnapshot(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    phase: 'ready',
    installed_version: '0.2.7',
    available_version: '0.2.7',
    install_dir: 'C:\\Voxtera',
    local_build_valid: true,
    operation: null,
    last_error: null,
    ...overrides,
  };
}

/* ---------- Tests ---------- */

const tokensCss = readFileSync(resolve(process.cwd(), 'src/styles/tokens.css'), 'utf8');

describe('App', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockInvoke.mockResolvedValue(makeSnapshot());
    mockListen.mockResolvedValue(() => {});
  });
  afterEach(() => cleanup());

  it('renders the Voxtera launcher shell in Portuguese', async () => {
    render(<App />);
    expect(await screen.findByAltText('')).toBeVisible(); // logo image
    expect(screen.getByText('Launcher oficial')).toBeVisible();
  });

  it('shows Jogar button when phase is ready', async () => {
    mockInvoke.mockResolvedValue(makeSnapshot({ phase: 'ready' }));
    render(<App />);
    expect(await screen.findByRole('button', { name: /Jogar/ })).toBeVisible();
  });

  it('shows Instalar button when phase is needs_install', async () => {
    mockInvoke.mockResolvedValue(makeSnapshot({ phase: 'needs_install', installed_version: null }));
    render(<App />);
    expect(await screen.findByRole('button', { name: /Instalar/ })).toBeVisible();
  });

  it('shows Atualizar button when phase is update_available', async () => {
    mockInvoke.mockResolvedValue(
      makeSnapshot({ phase: 'update_available', installed_version: '0.2.6', available_version: '0.2.7' }),
    );
    render(<App />);
    expect(await screen.findByRole('button', { name: /Atualizar/ })).toBeVisible();
  });

  it('shows Verificar arquivos when phase is repair_required', async () => {
    mockInvoke.mockResolvedValue(makeSnapshot({ phase: 'repair_required' }));
    render(<App />);
    expect(await screen.findByRole('button', { name: /Verificar arquivos/ })).toBeVisible();
  });

  it('invokes launch_game when Jogar is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(makeSnapshot({ phase: 'ready' }));
    render(<App />);
    const playButton = await screen.findByRole('button', { name: /Jogar/ });
    mockInvoke.mockResolvedValueOnce(42);
    await user.click(playButton);
    expect(mockInvoke).toHaveBeenCalledWith('launch_game');
  });

  it('invokes install_or_update when Instalar is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(makeSnapshot({ phase: 'needs_install', installed_version: null }));
    render(<App />);
    const installButton = await screen.findByRole('button', { name: /Instalar/ });
    mockInvoke.mockResolvedValueOnce(makeSnapshot({ phase: 'ready' }));
    await user.click(installButton);
    expect(mockInvoke).toHaveBeenCalledWith('install_or_update');
  });

  it('invokes scan_repair when Verificar arquivos is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(makeSnapshot({ phase: 'repair_required' }));
    render(<App />);
    const scanButton = await screen.findByRole('button', { name: /Verificar arquivos/ });
    mockInvoke.mockResolvedValueOnce({
      checked_files: 10,
      checked_bytes: 1000000,
      invalid_files: [],
      invalid_bytes: 0,
      deletions: [],
    });
    await user.click(scanButton);
    expect(mockInvoke).toHaveBeenCalledWith('scan_repair');
  });

  it('displays error from snapshot.last_error', async () => {
    mockInvoke.mockResolvedValueOnce(
      makeSnapshot({ phase: 'error', last_error: { code: 'network', message: 'Falha de conexao' } }),
    );
    render(<App />);
    expect(await screen.findByText(/Falha de conexao/)).toBeVisible();
  });

  it('shows settings panel when settings icon is clicked', async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByAltText('');
    const settingsButton = screen.getByTitle('Configurações');
    await user.click(settingsButton);
    expect(await screen.findByRole('dialog', { name: 'Configurações' })).toBeVisible();
  });
});

describe('shared brand tokens', () => {
  it('keeps the launcher CSS synchronized with the canonical JSON values', () => {
    const expectedCssVariables = {
      '--color-void': brandTokens.color.void,
      '--color-stone': brandTokens.color.stone,
      '--color-stone-raised': brandTokens.color.stoneRaised,
      '--color-ice': brandTokens.color.ice,
      '--color-ice-strong': brandTokens.color.iceStrong,
      '--color-gold': brandTokens.color.gold,
      '--color-text': brandTokens.color.text,
      '--color-muted': brandTokens.color.muted,
      '--color-danger': brandTokens.color.danger,
      '--radius-small': `${brandTokens.radius.small}px`,
      '--radius-medium': `${brandTokens.radius.medium}px`,
      '--motion-fast': `${brandTokens.motion.fastMs}ms`,
      '--motion-normal': `${brandTokens.motion.normalMs}ms`,
    };

    const cssVariables = Object.fromEntries(
      [...tokensCss.matchAll(/^\s*(--[\w-]+):\s*([^;]+);/gm)].map(([, name, value]) => [
        name,
        value,
      ]),
    );

    expect(cssVariables).toEqual(expectedCssVariables);
  });
});
