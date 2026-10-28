import { describe, expect, it } from 'vitest';
import type { JarvisSession, JarvisStatus } from '../data/jarvis-session';
import { required } from '../testing/required';
import { DrawList, type DrawItem } from './draw-list';
import { buildFrame, type FrameInput } from './frame-builder';
import { STATUS_COLORS, STRUCTURE_COLOR } from './palette';
import { Projector } from './projection';
import { SceneRoster } from './roster';

const VIEWPORT = { width: 1200, height: 760 };
const PULSE_WIDTH = 1.5;
const SETTLED = 10_000;

function session(id: string, repo: string, status: JarvisStatus = 'idle'): JarvisSession {
  return { id, repo, name: id, status, action: null, branch: null, lastActivityAt: null };
}

function frame(sessions: JarvisSession[], overrides: Partial<FrameInput> = {}, roster = new SceneRoster()) {
  if (overrides.entries === undefined) roster.update(sessions, 0);
  const now = overrides.now ?? SETTLED;
  const projector = new Projector();
  projector.configure(VIEWPORT, { yaw: 0.55, pitch: 0.46, zoom: 1 });
  const list = new DrawList();
  const overlay = buildFrame(
    {
      projector,
      viewport: VIEWPORT,
      entries: roster.entries(now),
      roster,
      rules: [],
      clock: 2,
      now,
      yaw: 0.55,
      reducedMotion: false,
      selectedId: null,
      balloonHeights: new Map(),
      headerBottom: 80,
      ...overrides,
    },
    list,
  );
  return { overlay, items: list.items.slice(0, list.count) };
}

function circles(items: DrawItem[], kind: 'dots' | 'rings', color: string, width?: number): number {
  return items
    .filter((item) => item.kind === kind && item.color === color && (width === undefined || item.width === width))
    .reduce((total, item) => total + item.length / 3, 0);
}

function segments(item: DrawItem | undefined): number {
  return item ? item.length / 4 : 0;
}

const coreItems = (items: DrawItem[]) => items.filter((item) => item.glow && item.color === STRUCTURE_COLOR && item.length > 0);

describe('buildFrame', () => {
  it('draws the floor as three rings of 72 segments and 16 rays', () => {
    const { items } = frame([session('a', 'web')]);
    expect(items[0]?.color).toBe(STRUCTURE_COLOR);
    expect(items[0]?.alpha).toBe(0.16);
    expect(segments(items[0])).toBe(3 * 72 + 16);
  });

  it('dims the floor and the core in an empty scene', () => {
    const { items, overlay } = frame([]);
    expect(items[0]?.alpha).toBeLessThan(0.16);
    expect(coreItems(items)).toHaveLength(1);
    expect(coreItems(items)[0]?.alpha).toBe(0.5);
    expect(overlay.labels).toEqual([]);
    expect(overlay.balloons).toEqual([]);
  });

  it('shows the core with two or more projects but not with a single one', () => {
    expect(coreItems(frame([session('a', 'web')]).items)).toHaveLength(0);
    const two = coreItems(frame([session('a', 'web'), session('b', 'api')]).items);
    expect(two).toHaveLength(1);
    expect(two[0]?.alpha).toBe(0.9);
  });

  it('colors each shape with its most urgent session and gives it a glow', () => {
    const { items } = frame([session('a', 'web', 'working'), session('b', 'web', 'waiting'), session('c', 'api', 'done')]);
    const shapes = items.filter((item) => item.glow && item.color !== STRUCTURE_COLOR);
    expect(shapes.map((item) => item.color)).toEqual([STATUS_COLORS.waiting, STATUS_COLORS.done]);
    expect(shapes.every((item) => item.width === 1.3 && segments(item) > 0)).toBe(true);
  });

  it('raises a scan ring while a project forms and removes it once formed', () => {
    const scanLength = (now: number) => frame([session('a', 'web')], { now }).items.filter((item) => item.width === 2).reduce((total, item) => total + item.length, 0);
    expect(scanLength(450)).toBeGreaterThan(0);
    expect(scanLength(SETTLED)).toBe(0);
  });

  it('leaves projects of the opening sequence out until their turn', () => {
    const { overlay } = frame([session('a', 'web'), session('b', 'api')], { now: 100 });
    expect(overlay.labels.map((label) => label.repo)).toEqual(['web']);
  });

  it('fades a project out with its last session', () => {
    const roster = new SceneRoster();
    roster.update([session('a', 'web', 'done')], 0);
    roster.update([], 5_000);
    const { items, overlay } = frame([], { entries: roster.entries(5_150), now: 5_150 }, roster);
    const shape = items.find((item) => item.glow && item.color === STATUS_COLORS.done);
    expect(shape?.alpha).toBeCloseTo(0.95 * 0.5);
    expect(overlay.balloons[0]?.opacity).toBeCloseTo(0.75);
  });

  it('draws one satellite per session in its status color', () => {
    const { items } = frame([session('a', 'web', 'working'), session('b', 'web', 'waiting'), session('c', 'api', 'done'), session('d', 'api', 'idle')]);
    for (const status of ['working', 'waiting', 'done', 'idle'] as const) expect(circles(items, 'dots', STATUS_COLORS[status])).toBe(1);
  });

  it('spins an electron around working satellites and pulses waiting ones', () => {
    const { items } = frame([session('a', 'web', 'working'), session('b', 'web', 'waiting')]);
    expect(circles(items, 'dots', STRUCTURE_COLOR)).toBe(1);
    expect(circles(items, 'rings', STATUS_COLORS.waiting, PULSE_WIDTH)).toBe(1);
  });

  it('drops electrons, packets and pulses when motion is reduced', () => {
    const { items } = frame([session('a', 'web', 'working'), session('b', 'api', 'waiting')], { reducedMotion: true });
    expect(circles(items, 'dots', STRUCTURE_COLOR)).toBe(0);
    expect(circles(items, 'rings', STATUS_COLORS.waiting, PULSE_WIDTH)).toBe(0);
  });

  it('links each project to the core and runs two packets on links of working projects', () => {
    const { items } = frame([session('a', 'web', 'working'), session('b', 'api', 'done')]);
    const links = items.find((item) => item.dash === 'link');
    expect(segments(links)).toBe(2);
    expect(circles(items, 'dots', STRUCTURE_COLOR)).toBe(2 + 1);
  });

  it('labels each project below its base with the number of sessions', () => {
    const { overlay } = frame([session('a', 'web'), session('b', 'web')]);
    expect(overlay.labels).toHaveLength(1);
    expect(overlay.labels[0]).toMatchObject({ repo: 'web', sessionCount: 2, opacity: 1 });
    expect(overlay.labels[0]?.y).toBeGreaterThan(456);
  });

  it('places one balloon per session above its satellite and links them with a leader of the status color', () => {
    const { items, overlay } = frame([session('a', 'web', 'working'), session('b', 'api', 'done')]);
    expect(overlay.balloons.map((balloon) => balloon.id).sort()).toEqual(['a', 'b']);
    for (const status of ['working', 'done'] as const) {
      const leader = items.find((item) => item.kind === 'lines' && item.color === STATUS_COLORS[status] && !item.glow);
      expect(segments(leader)).toBe(1);
      expect(circles(items, 'rings', STATUS_COLORS[status], 1)).toBe(1);
    }
  });

  it('keeps balloons below the header', () => {
    const { overlay } = frame([session('a', 'web')], { headerBottom: 700, balloonHeights: new Map([['a', 120]]) });
    expect(required(overlay.balloons[0]).bottom - 120).toBeGreaterThanOrEqual(700);
  });

  it('knows which way to turn to bring each session forward', () => {
    expect(frame([session('a', 'web')]).overlay.turnAngles.get('a')).toBeNull();
    const ring = frame([session('a', 'web'), session('b', 'api')]).overlay.turnAngles;
    expect(ring.get('a')).toBeCloseTo(-Math.PI / 2);
    expect(ring.get('b')).toBeCloseTo(Math.PI / 2);
  });
});
