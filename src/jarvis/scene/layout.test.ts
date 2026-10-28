import { describe, expect, it } from 'vitest';
import type { JarvisSession, JarvisStatus } from '../data/jarvis-session';
import { required } from '../testing/required';
import { layoutProjects, satelliteOffset } from './layout';

function session(id: string, repo: string, status: JarvisStatus = 'idle'): JarvisSession {
  return { id, repo, name: id, status, action: null, branch: null, lastActivityAt: null };
}

describe('layoutProjects', () => {
  it('keeps the core alone in an empty scene', () => {
    expect(layoutProjects([])).toEqual({ projects: [], hasCore: true });
  });

  it('puts a single project in the centre, 20% larger and without the core', () => {
    const layout = layoutProjects([session('a', 'web')]);
    expect(layout.hasCore).toBe(false);
    expect(layout.projects).toHaveLength(1);
    const project = required(layout.projects[0]);
    expect(project.x).toBeCloseTo(0);
    expect(project.z).toBeCloseTo(0);
    expect(project.scale).toBe(1.2);
    expect(project.outward).toBeCloseTo(-Math.PI / 2);
  });

  it('spreads two projects on a 210 ring, starting at the back', () => {
    const layout = layoutProjects([session('a', 'web'), session('b', 'api')]);
    expect(layout.hasCore).toBe(true);
    expect(layout.projects.map((project) => [project.repo, Math.round(project.x), Math.round(project.z)])).toEqual([
      ['web', 0, -210],
      ['api', 0, 210],
    ]);
  });

  it('uses a 260 ring from three projects on, in order of first appearance', () => {
    const layout = layoutProjects([session('a', 'web'), session('b', 'api'), session('c', 'web'), session('d', 'mobile')]);
    expect(layout.projects.map((project) => project.repo)).toEqual(['web', 'api', 'mobile']);
    const second = required(layout.projects[1]);
    expect(second.x).toBeCloseTo(260 * Math.cos(-Math.PI / 2 + (2 * Math.PI) / 3));
    expect(second.z).toBeCloseTo(260 * Math.sin(-Math.PI / 2 + (2 * Math.PI) / 3));
    expect(second.outward).toBeCloseTo(-Math.PI / 2 + (2 * Math.PI) / 3);
    expect(layout.projects[0]?.sessions.map((item) => item.id)).toEqual(['a', 'c']);
  });

  it('colors a project by its most urgent session: waiting, working, done, idle', () => {
    const colorOf = (...statuses: JarvisStatus[]) => layoutProjects(statuses.map((status, index) => session(String(index), 'web', status))).projects[0]?.status;
    expect(colorOf('idle', 'working', 'waiting', 'done')).toBe('waiting');
    expect(colorOf('done', 'working')).toBe('working');
    expect(colorOf('idle', 'done')).toBe('done');
    expect(colorOf('idle')).toBe('idle');
  });

  it('marks projects with a working session so their shape spins faster', () => {
    const layout = layoutProjects([session('a', 'web', 'waiting'), session('b', 'web', 'working'), session('c', 'api', 'done')]);
    expect(layout.projects.map((project) => project.working)).toEqual([true, false]);
  });
});

describe('satelliteOffset', () => {
  it('places a lone session on the orbit, facing away from the centre', () => {
    const project = required(layoutProjects([session('a', 'web')]).projects[0]);
    const offset = satelliteOffset(project, 0, 1);
    expect(offset.x).toBeCloseTo(0);
    expect(offset.z).toBeCloseTo(-112 * 1.2);
  });

  it('fans sessions 0.95 rad apart around the outward direction', () => {
    const project = required(layoutProjects([session('a', 'web'), session('b', 'api')]).projects[0]);
    const first = satelliteOffset(project, 0, 2);
    const second = satelliteOffset(project, 1, 2);
    expect(first.x).toBeCloseTo(112 * Math.cos(-Math.PI / 2 - 0.475));
    expect(first.z).toBeCloseTo(112 * Math.sin(-Math.PI / 2 - 0.475));
    expect(second.x).toBeCloseTo(112 * Math.cos(-Math.PI / 2 + 0.475));
  });
});
