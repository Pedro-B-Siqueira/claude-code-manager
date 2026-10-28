import type { JarvisSession } from '../data/jarvis-session';

export interface RosterEntry {
  session: JarvisSession;
  bornAt: number;
  /** When the session closed; it stays on screen while it fades out. */
  leftAt: number | null;
}

const PROJECT_RISE_MS = 900;
const SESSION_DELAY_MS = 550;
const OPENING_STAGGER_MS = 500;
const SESSION_APPEAR_MS = 700;
const SESSION_FADE_MS = 300;

function clampUnit(value: number): number {
  return value < 0 ? 0 : value > 1 ? 1 : value;
}

function easeOutCubic(progress: number): number {
  return 1 - (1 - progress) ** 3;
}

/**
 * Who is on stage and since when. The first update is the opening: projects that already exist enter
 * one at a time. Later projects form as soon as they appear.
 */
export class SceneRoster {
  private readonly sessions = new Map<string, RosterEntry>();
  private readonly projectBirths = new Map<string, number>();
  private opened = false;

  update(current: readonly JarvisSession[], now: number): void {
    const opening = !this.opened;
    this.opened = true;
    let staggered = 0;
    for (const session of current) {
      if (!this.projectBirths.has(session.repo)) {
        this.projectBirths.set(session.repo, opening ? now + staggered * OPENING_STAGGER_MS : now);
        staggered += 1;
      }
      const existing = this.sessions.get(session.id);
      if (existing) {
        existing.session = session;
        existing.leftAt = null;
      } else {
        const projectBorn = this.projectBirths.get(session.repo) ?? now;
        this.sessions.set(session.id, { session, bornAt: Math.max(now, projectBorn + SESSION_DELAY_MS), leftAt: null });
      }
    }
    const present = new Set(current.map((session) => session.id));
    for (const [id, entry] of this.sessions) {
      if (!present.has(id) && entry.leftAt === null) entry.leftAt = now;
    }
  }

  /** Sessions to draw, in order of arrival. Faded-out sessions and empty projects are dropped here. */
  entries(now: number): RosterEntry[] {
    for (const [id, entry] of this.sessions) {
      if (entry.leftAt !== null && now - entry.leftAt >= SESSION_FADE_MS) this.sessions.delete(id);
    }
    const repos = new Set<string>();
    for (const entry of this.sessions.values()) repos.add(entry.session.repo);
    for (const repo of this.projectBirths.keys()) {
      if (!repos.has(repo)) this.projectBirths.delete(repo);
    }
    return [...this.sessions.values()];
  }

  projectBornAt(repo: string): number | undefined {
    return this.projectBirths.get(repo);
  }

  /** How far the project's shape has risen from the floor, eased. */
  projectRise(repo: string, now: number): number {
    const born = this.projectBirths.get(repo);
    return born === undefined ? 0 : easeOutCubic(clampUnit((now - born) / PROJECT_RISE_MS));
  }

  opacity(entry: RosterEntry, now: number): number {
    const appear = clampUnit((now - entry.bornAt) / SESSION_APPEAR_MS);
    const fade = entry.leftAt === null ? 1 : clampUnit(1 - (now - entry.leftAt) / SESSION_FADE_MS);
    return appear * fade;
  }

  isAnimating(now: number): boolean {
    for (const entry of this.sessions.values()) {
      if (entry.leftAt !== null && now - entry.leftAt < SESSION_FADE_MS) return true;
      if (entry.leftAt === null && now - entry.bornAt < SESSION_APPEAR_MS) return true;
      if (this.projectRise(entry.session.repo, now) < 1) return true;
    }
    return false;
  }
}
