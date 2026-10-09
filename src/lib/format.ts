const compactNumber = new Intl.NumberFormat('pt-BR', {
  notation: 'compact',
  maximumFractionDigits: 1,
});

const usdCurrency = new Intl.NumberFormat('pt-BR', {
  style: 'currency',
  currency: 'USD',
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

export function formatTokens(tokens: number): string {
  return compactNumber.format(tokens);
}

export function formatCost(costUsd: number): string {
  return usdCurrency.format(costUsd);
}

export function formatMemory(megabytes: number): string {
  if (megabytes >= 1024) return `${(megabytes / 1024).toFixed(1).replace('.', ',')} GB`;
  return `${Math.round(megabytes)} MB`;
}

export function fileName(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path;
}

const relativeTime = new Intl.RelativeTimeFormat('pt-BR', { numeric: 'auto' });

const RELATIVE_STEPS: ReadonlyArray<{ unit: Intl.RelativeTimeFormatUnit; milliseconds: number }> = [
  { unit: 'year', milliseconds: 365 * 24 * 60 * 60 * 1000 },
  { unit: 'month', milliseconds: 30 * 24 * 60 * 60 * 1000 },
  { unit: 'week', milliseconds: 7 * 24 * 60 * 60 * 1000 },
  { unit: 'day', milliseconds: 24 * 60 * 60 * 1000 },
  { unit: 'hour', milliseconds: 60 * 60 * 1000 },
  { unit: 'minute', milliseconds: 60 * 1000 },
];

export function formatRelativeTime(timestampMs: number | null, nowMs: number = Date.now()): string {
  if (timestampMs === null) return '—';
  const elapsed = timestampMs - nowMs;
  for (const step of RELATIVE_STEPS) {
    if (Math.abs(elapsed) >= step.milliseconds) {
      return relativeTime.format(Math.round(elapsed / step.milliseconds), step.unit);
    }
  }
  return 'agora';
}

export function formatDuration(durationMs: number | null): string {
  if (durationMs === null) return '—';
  const totalMinutes = Math.round(durationMs / 60_000);
  if (totalMinutes < 1) return 'menos de 1 min';
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours === 0) return `${minutes} min`;
  return minutes === 0 ? `${hours} h` : `${hours} h ${minutes} min`;
}

export function relativePath(path: string, base: string | null): string {
  if (base && path.startsWith(`${base}/`)) return path.slice(base.length + 1);
  return path;
}
