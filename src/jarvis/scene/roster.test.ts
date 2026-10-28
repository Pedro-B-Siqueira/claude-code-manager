import { describe, expect, it } from 'vitest';
import type { JarvisSession } from '../data/jarvis-session';
import { required } from '../testing/required';
import { SceneRoster } from './roster';

function session(id: string, repo: string): JarvisSession {
  return { id, repo, name: id, status: 'working', action: null, branch: null, lastActivityAt: null };
}

const OPENING = [session('a', 'web'), session('b', 'api'), session('c', 'web')];

describe('SceneRoster', () => {
  it('brings projects that already exist in one at a time, 500 ms apart, each session 550 ms after its project', () => {
    const roster = new SceneRoster();
    roster.update(OPENING, 1_000);
    expect(roster.projectBornAt('web')).toBe(1_000);
    expect(roster.projectBornAt('api')).toBe(1_500);
    expect(roster.entries(1_000).map((entry) => [entry.session.id, entry.bornAt])).toEqual([
      ['a', 1_550],
      ['b', 2_050],
      ['c', 1_550],
    ]);
  });

  it('raises a project over 900 ms with a cubic ease-out', () => {
    const roster = new SceneRoster();
    roster.update(OPENING, 1_000);
    expect(roster.projectRise('web', 1_000)).toBe(0);
    expect(roster.projectRise('web', 1_450)).toBeCloseTo(0.875);
    expect(roster.projectRise('web', 1_900)).toBe(1);
    expect(roster.projectRise('api', 1_400)).toBe(0);
  });

  it('forms a project that appears later right away, not in the opening sequence', () => {
    const roster = new SceneRoster();
    roster.update(OPENING, 1_000);
    roster.update([...OPENING, session('d', 'mobile'), session('e', 'api')], 5_000);
    expect(roster.projectBornAt('mobile')).toBe(5_000);
    const born = Object.fromEntries(roster.entries(5_000).map((entry) => [entry.session.id, entry.bornAt]));
    expect(born.d).toBe(5_550);
    expect(born.e).toBe(5_000);
  });

  it('fades a closed session out over 300 ms, then drops it', () => {
    const roster = new SceneRoster();
    roster.update(OPENING, 0);
    roster.update([session('a', 'web'), session('b', 'api')], 3_000);
    const leaving = roster.entries(3_150).find((entry) => entry.session.id === 'c');
    expect(leaving?.leftAt).toBe(3_000);
    expect(leaving && roster.opacity(leaving, 3_150)).toBeCloseTo(0.5);
    expect(roster.entries(3_300).map((entry) => entry.session.id)).toEqual(['a', 'b']);
  });

  it('keeps a session that comes back before its fade ends', () => {
    const roster = new SceneRoster();
    roster.update(OPENING, 0);
    roster.update([session('a', 'web')], 3_000);
    roster.update(OPENING, 3_100);
    const back = roster.entries(3_400).find((entry) => entry.session.id === 'c');
    expect(back?.leftAt).toBeNull();
    expect(back && roster.opacity(back, 3_400)).toBe(1);
  });

  it('shows the latest data of sessions that stay', () => {
    const roster = new SceneRoster();
    roster.update(OPENING, 0);
    roster.update([{ ...session('a', 'web'), status: 'done' }], 3_000);
    expect(roster.entries(3_000).find((entry) => entry.session.id === 'a')?.session.status).toBe('done');
  });

  it('grows a new satellite in over 700 ms', () => {
    const roster = new SceneRoster();
    roster.update([session('a', 'web')], 0);
    const entry = required(roster.entries(0)[0]);
    expect(roster.opacity(entry, 550)).toBe(0);
    expect(roster.opacity(entry, 900)).toBeCloseTo(0.5);
    expect(roster.opacity(entry, 1_300)).toBe(1);
  });

  it('forgets a project once its last session is gone, so it forms again when it returns', () => {
    const roster = new SceneRoster();
    roster.update([session('a', 'web')], 0);
    roster.update([], 2_000);
    roster.entries(2_400);
    expect(roster.projectBornAt('web')).toBeUndefined();
    roster.update([session('a', 'web')], 9_000);
    expect(roster.projectBornAt('web')).toBe(9_000);
  });

  it('reports whether anything is still moving so a still scene can stop drawing', () => {
    const roster = new SceneRoster();
    roster.update([session('a', 'web')], 0);
    expect(roster.isAnimating(500)).toBe(true);
    expect(roster.isAnimating(1_300)).toBe(false);
    roster.update([], 2_000);
    expect(roster.isAnimating(2_100)).toBe(true);
    expect(roster.isAnimating(2_400)).toBe(false);
  });
});
