import type { Theme } from '../api/types';

const THEME_TRANSITION_CLASS = 'theme-transition';
const THEME_TRANSITION_MS = 400;

export function applyTheme(theme: Theme, animate: boolean): void {
  const root = document.documentElement;
  if (animate && !prefersReducedMotion()) {
    root.classList.add(THEME_TRANSITION_CLASS);
    window.setTimeout(() => root.classList.remove(THEME_TRANSITION_CLASS), THEME_TRANSITION_MS);
  }
  root.dataset.theme = theme;
}

function prefersReducedMotion(): boolean {
  return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
}
