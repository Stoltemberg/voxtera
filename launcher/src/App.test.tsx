import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import brandTokens from '../../brand/tokens.json';
import { App } from './App';

const tokensCss = readFileSync(resolve(process.cwd(), 'src/styles/tokens.css'), 'utf8');

describe('App', () => {
  it('renders the Voxtera launcher shell in Portuguese', () => {
    render(<App />);
    expect(screen.getByRole('heading', { name: 'Voxtera' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Verificar instalação' })).toBeEnabled();
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
