export type SessionSourceKind = 'live' | 'mock';

export interface LaunchOptions {
  source: SessionSourceKind;
  /** Canvas backing-scale cap for measurements; null keeps the scene's default. */
  pixelRatio: number | null;
  /** Points to keep clear at the top: the notch band covers them over full-screen spaces. */
  safeTop: number;
}

const PIXEL_RATIO_RANGE = [0.5, 3] as const;

function parsePixelRatio(value: string | null): number | null {
  const ratio = value === null || value.trim() === '' ? Number.NaN : Number(value);
  if (!Number.isFinite(ratio)) return null;
  return Math.min(PIXEL_RATIO_RANGE[1], Math.max(PIXEL_RATIO_RANGE[0], ratio));
}

/** `source` and `pixelRatio` come only in development builds (`CCM_JARVIS_MOCK`, `CCM_JARVIS_PIXEL_RATIO`); `safeTop` always. */
export function launchOptions(search: string): LaunchOptions {
  const parameters = new URLSearchParams(search);
  return {
    source: parameters.get('source') === 'mock' ? 'mock' : 'live',
    pixelRatio: parsePixelRatio(parameters.get('pixelRatio')),
    safeTop: Math.max(0, Number(parameters.get('safeTop')) || 0),
  };
}

const ACTION_MAX_CHARS = 96;

export function clipAction(action: string): string {
  const flat = action.replace(/\s*\n\s*/g, ' ').trim();
  return flat.length > ACTION_MAX_CHARS ? `${flat.slice(0, ACTION_MAX_CHARS - 1)}…` : flat;
}
