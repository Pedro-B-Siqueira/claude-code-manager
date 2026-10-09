import type { PaletteAction } from '../palette/search';
import { shortcutLabel } from './shortcuts';

export const PALETTE_ACTIONS: PaletteAction[] = [
  { id: 'new-session', label: 'Nova sessão', keywords: 'abrir terminal claude worktree', shortcut: shortcutLabel('new-session') },
  { id: 'resume', label: 'Retomar sessão', keywords: 'histórico biblioteca resume', shortcut: null },
  { id: 'needs-you', label: 'Mostrar as que precisam de você', keywords: 'permissão esperando filtro', shortcut: null },
  { id: 'grid', label: 'Ver em grade', keywords: 'cards layout', shortcut: null },
  { id: 'focus', label: 'Ver em foco', keywords: 'terminal abas', shortcut: null },
  { id: 'theme', label: 'Alternar tema claro/escuro', keywords: 'aparência dark light', shortcut: null },
  { id: 'settings', label: 'Configurações', keywords: 'preferências opções', shortcut: shortcutLabel('settings') },
];
