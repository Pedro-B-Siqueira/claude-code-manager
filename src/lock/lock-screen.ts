import { formatClock, type LockedSummary } from '../jarvis/data/lock-text';

export interface LockScreen {
  root: HTMLElement;
  update: (summary: LockedSummary, now: Date) => void;
}

const LOCK_ICON = 'M6 11h12v9H6ZM8.5 11V8a3.5 3.5 0 0 1 7 0v3M12 15v2';

function element<K extends keyof HTMLElementTagNameMap>(document: Document, tag: K, className: string, text = ''): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function lockIcon(document: Document): SVGSVGElement {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('class', 'icon');
  svg.setAttribute('aria-hidden', 'true');
  const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  path.setAttribute('d', LOCK_ICON);
  svg.append(path);
  return svg;
}

/** The simple screen on extra displays during Repouso: no hologram, just enough to know what is going on. */
export function createLockScreen(document: Document): LockScreen {
  const root = element(document, 'main', 'lock');
  const clock = element(document, 'p', 'clock');
  const sessions = element(document, 'span', 'sessions');
  const waiting = element(document, 'span', 'waiting');
  const summary = element(document, 'p', 'summary');
  summary.append(sessions, waiting);
  root.append(
    lockIcon(document),
    clock,
    element(document, 'h1', 'title', 'Bloqueado pelo Claude Code Manager'),
    summary,
    element(document, 'p', 'hint', 'Desbloqueie na tela principal com o Touch ID ou a senha do Mac.'),
  );
  return {
    root,
    update: (current, now) => {
      clock.textContent = formatClock(now);
      sessions.textContent = current.sessions;
      waiting.textContent = current.waiting ?? '';
      waiting.hidden = current.waiting === null;
    },
  };
}
