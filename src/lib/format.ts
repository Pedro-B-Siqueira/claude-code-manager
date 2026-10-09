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
