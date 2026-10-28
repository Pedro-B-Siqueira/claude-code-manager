import type { JarvisStatus } from '../data/jarvis-session';

/** Blue and orange differ in luminance too, on purpose; do not swap them for red and green. */
export const STATUS_COLORS: Record<JarvisStatus, string> = {
  working: '#3D99E9',
  waiting: '#FF9800',
  done: '#24B787',
  idle: '#9E9E9E',
};

/** Floor, links, core, orbits and moving sparks. */
export const STRUCTURE_COLOR = '#6699FF';
