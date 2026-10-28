export type ShapeKind = 'tower' | 'eye' | 'fortress' | 'orb' | 'crystal';

export const SHAPE_KINDS: readonly ShapeKind[] = ['tower', 'eye', 'fortress', 'orb', 'crystal'];

/** `match` is a case-insensitive glob where `*` is the only wildcard: `api*`, `*eye*`, `web`. */
export interface ShapeRule {
  match: string;
  shape: ShapeKind;
}

export interface ShapeChoice {
  kind: ShapeKind;
  /** Crystals vary deterministically by name so repos without a rule still look distinct. */
  variant: number;
}

export const CRYSTAL_VARIANTS = 3;

function escapeRegExp(text: string): string {
  return text.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
}

function globMatches(pattern: string, name: string): boolean {
  const source = pattern.split('*').map(escapeRegExp).join('.*');
  return new RegExp(`^${source}$`, 'i').test(name);
}

/** FNV-1a (32 bits) over the UTF-8 bytes. */
function hashName(name: string): number {
  let hash = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(name)) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash >>> 0;
}

export function shapeFor(repoName: string, rules: readonly ShapeRule[]): ShapeChoice {
  const rule = rules.find((candidate) => globMatches(candidate.match, repoName));
  if (rule) return { kind: rule.shape, variant: 0 };
  return { kind: 'crystal', variant: hashName(repoName.toLowerCase()) % CRYSTAL_VARIANTS };
}
