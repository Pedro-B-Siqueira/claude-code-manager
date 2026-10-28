import { describe, expect, it } from 'vitest';
import { shapeFor, type ShapeRule } from './shape-rules';

const RULES: ShapeRule[] = [
  { match: 'forge*', shape: 'tower' },
  { match: '*eye*', shape: 'eye' },
  { match: 'keep', shape: 'fortress' },
  { match: '*-orb', shape: 'orb' },
];

describe('shapeFor', () => {
  it('matches prefix, contains, exact and suffix patterns', () => {
    expect(shapeFor('forge-2', RULES).kind).toBe('tower');
    expect(shapeFor('my-eye-app', RULES).kind).toBe('eye');
    expect(shapeFor('keep', RULES).kind).toBe('fortress');
    expect(shapeFor('design-orb', RULES).kind).toBe('orb');
  });

  it('ignores case in both the pattern and the repository name', () => {
    expect(shapeFor('FORGE', RULES).kind).toBe('tower');
    expect(shapeFor('keep', [{ match: 'KEEP', shape: 'fortress' }]).kind).toBe('fortress');
  });

  it('does not treat a pattern without stars as "contains"', () => {
    expect(shapeFor('keeper', RULES).kind).toBe('crystal');
    expect(shapeFor('old-forge', RULES).kind).toBe('crystal');
  });

  it('uses the first rule that matches', () => {
    expect(shapeFor('forge-eye', RULES).kind).toBe('tower');
  });

  it('treats regex characters in patterns as plain text', () => {
    expect(shapeFor('a.b', [{ match: 'a.b', shape: 'orb' }]).kind).toBe('orb');
    expect(shapeFor('axb', [{ match: 'a.b', shape: 'orb' }]).kind).toBe('crystal');
  });

  it('falls back to a crystal whose variant is a stable hash of the lowercase name', () => {
    expect(shapeFor('dotfiles', [])).toEqual({ kind: 'crystal', variant: 0 });
    expect(shapeFor('notes', [])).toEqual({ kind: 'crystal', variant: 1 });
    expect(shapeFor('Notes', [])).toEqual({ kind: 'crystal', variant: 1 });
    expect(shapeFor('shop', [])).toEqual({ kind: 'crystal', variant: 2 });
  });

  it('gives matched shapes the base variant', () => {
    expect(shapeFor('forge-2', RULES)).toEqual({ kind: 'tower', variant: 0 });
  });
});
