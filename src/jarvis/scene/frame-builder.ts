import { STATUS_ORDER, type JarvisStatus } from '../data/jarvis-session';
import { layoutBalloons, type BalloonPlacement } from './balloon-layout';
import type { DrawItem, DrawList } from './draw-list';
import { layoutProjects, ORBIT_HEIGHT, ORBIT_RADIUS, satelliteOffset, type ProjectSlot } from './layout';
import { STATUS_COLORS, STRUCTURE_COLOR } from './palette';
import type { Projector, Viewport } from './projection';
import type { RosterEntry, SceneRoster } from './roster';
import { shapeFor, type ShapeRule } from './shape-rules';
import { shapeGeometry, type ShapeGeometry } from './shapes';

export interface FrameInput {
  /** Already configured for this frame's viewport and pose. */
  projector: Projector;
  viewport: Viewport;
  entries: readonly RosterEntry[];
  roster: SceneRoster;
  rules: readonly ShapeRule[];
  /** Animation clock in seconds; it stands still when motion is reduced. */
  clock: number;
  now: number;
  yaw: number;
  reducedMotion: boolean;
  selectedId: string | null;
  balloonHeights: ReadonlyMap<string, number>;
  headerBottom: number;
}

export interface ProjectLabel {
  repo: string;
  sessionCount: number;
  x: number;
  y: number;
  opacity: number;
}

export interface FrameOverlay {
  labels: ProjectLabel[];
  balloons: BalloonPlacement[];
  /** Ring angle of each session's project, to turn it to the front; null for a lone centred project. */
  turnAngles: Map<string, number | null>;
}

interface SessionMark {
  id: string;
  status: JarvisStatus;
  opacity: number;
  x: number;
  y: number;
  radius: number;
  electron: { x: number; y: number; radius: number } | null;
  anchorX: number;
  anchorY: number;
  depth: number;
}

interface FrameLayers {
  orbits: DrawItem;
  links: DrawItem;
  packets: DrawItem;
  scan: DrawItem;
}

/** What one frame accumulates while it walks the projects. */
interface FrameState {
  input: FrameInput;
  list: DrawList;
  layers: FrameLayers;
  overlay: FrameOverlay;
  marks: SessionMark[];
  entryById: ReadonlyMap<string, RosterEntry>;
  /** Two or more projects: they sit on a ring around the core. */
  ring: boolean;
}

const FLOOR_OUTER = 440;
const FLOOR_RADII = [150, 300, FLOOR_OUTER] as const;
const FLOOR_SIDES = 72;
const FLOOR_RAYS = 16;
const RAY_START = 70;
const LINK_START = 34;
const LINK_END_GAP = 78;
const SPIN_WORKING = 0.35;
const SPIN_CALM = 0.06;
const BALLOON_LIFT = 78;
const DEFAULT_BALLOON_HEIGHT = 96;
const PULSE_SECONDS = 1.25;
const SHAPE_ALPHA = 0.95;

function fraction(value: number): number {
  return ((value % 1) + 1) % 1;
}

function drawSegment(item: DrawItem, projector: Projector, fromX: number, fromY: number, fromZ: number, toX: number, toY: number, toZ: number): void {
  projector.project(fromX, fromY, fromZ);
  const startX = projector.x;
  const startY = projector.y;
  projector.project(toX, toY, toZ);
  item.line(startX, startY, projector.x, projector.y);
}

function drawRing(item: DrawItem, projector: Projector, centerX: number, y: number, centerZ: number, radius: number, sides: number): void {
  projector.project(centerX + radius, y, centerZ);
  const firstX = projector.x;
  const firstY = projector.y;
  let previousX = firstX;
  let previousY = firstY;
  for (let side = 1; side < sides; side += 1) {
    const angle = (side / sides) * Math.PI * 2;
    projector.project(centerX + radius * Math.cos(angle), y, centerZ + radius * Math.sin(angle));
    item.line(previousX, previousY, projector.x, projector.y);
    previousX = projector.x;
    previousY = projector.y;
  }
  item.line(previousX, previousY, firstX, firstY);
}

function drawFloor(item: DrawItem, projector: Projector): void {
  for (const radius of FLOOR_RADII) drawRing(item, projector, 0, 0, 0, radius, FLOOR_SIDES);
  for (let ray = 0; ray < FLOOR_RAYS; ray += 1) {
    const angle = (ray / FLOOR_RAYS) * Math.PI * 2;
    drawSegment(item, projector, RAY_START * Math.cos(angle), 0, RAY_START * Math.sin(angle), FLOOR_OUTER * Math.cos(angle), 0, FLOOR_OUTER * Math.sin(angle));
  }
}

/** Small spinning octahedron in the middle of the floor: a square waist joined to a top and a bottom tip. */
function drawCore(item: DrawItem, projector: Projector, clock: number): void {
  const turn = clock * 0.6;
  for (let corner = 0; corner < 4; corner += 1) {
    const angle = turn + (corner / 4) * Math.PI * 2;
    const nextAngle = angle + Math.PI / 2;
    const x = 24 * Math.cos(angle);
    const z = 24 * Math.sin(angle);
    drawSegment(item, projector, x, 76, z, 24 * Math.cos(nextAngle), 76, 24 * Math.sin(nextAngle));
    drawSegment(item, projector, x, 76, z, 0, 112, 0);
    drawSegment(item, projector, x, 76, z, 0, 40, 0);
  }
}

function placeShapePoint(projector: Projector, project: ProjectSlot, x: number, y: number, z: number, cos: number, sin: number, height: number): void {
  projector.project(project.x + (x * cos - z * sin) * project.scale, y * height, project.z + (x * sin + z * cos) * project.scale);
}

function drawShape(item: DrawItem, input: FrameInput, geometry: ShapeGeometry, project: ProjectSlot, rise: number, spin: number): void {
  const { projector } = input;
  const spinCos = Math.cos(spin);
  const spinSin = Math.sin(spin);
  const faceCos = Math.cos(-input.yaw);
  const faceSin = Math.sin(-input.yaw);
  const height = rise * project.scale;
  const segment = geometry.segments;
  for (let index = 0; index < geometry.count; index += 1) {
    const facing = geometry.billboard[index] === 1;
    const cos = facing ? faceCos : spinCos;
    const sin = facing ? faceSin : spinSin;
    const at = index * 6;
    placeShapePoint(projector, project, segment[at] ?? 0, segment[at + 1] ?? 0, segment[at + 2] ?? 0, cos, sin, height);
    const startX = projector.x;
    const startY = projector.y;
    placeShapePoint(projector, project, segment[at + 3] ?? 0, segment[at + 4] ?? 0, segment[at + 5] ?? 0, cos, sin, height);
    item.line(startX, startY, projector.x, projector.y);
  }
}

function drawLink(layers: FrameLayers, input: FrameInput, project: ProjectSlot): void {
  const distance = Math.hypot(project.x, project.z) || 1;
  const unitX = project.x / distance;
  const unitZ = project.z / distance;
  const endGap = LINK_END_GAP * project.scale;
  drawSegment(layers.links, input.projector, unitX * LINK_START, 0, unitZ * LINK_START, project.x - unitX * endGap, 0, project.z - unitZ * endGap);
  if (!project.working || input.reducedMotion) return;
  for (let packet = 0; packet < 2; packet += 1) {
    const along = LINK_START + fraction(input.clock * 0.45 + packet * 0.5 + project.index * 0.21) * (distance - 112);
    input.projector.project(unitX * along, 2, unitZ * along);
    layers.packets.circle(input.projector.x, input.projector.y, 2.6 * input.projector.scale);
  }
}

function projectOpacity(entries: readonly RosterEntry[], input: FrameInput): number {
  if (entries.some((entry) => entry.leftAt === null)) return 1;
  return Math.max(0, ...entries.map((entry) => input.roster.opacity(entry, input.now)));
}

function markSession(input: FrameInput, project: ProjectSlot, entry: RosterEntry, index: number, rise: number): SessionMark {
  const { projector } = input;
  const offset = satelliteOffset(project, index, project.sessions.length);
  const x = project.x + offset.x;
  const y = ORBIT_HEIGHT * rise * project.scale;
  const z = project.z + offset.z;
  projector.project(x, y, z);
  const opacity = input.roster.opacity(entry, input.now);
  const mark = { id: entry.session.id, status: entry.session.status, opacity, x: projector.x, y: projector.y, radius: (5 * projector.perspective + 1.5) * projector.scale * Math.min(1, opacity * 1.5) };
  let electron: SessionMark['electron'] = null;
  if (entry.session.status === 'working' && !input.reducedMotion) {
    const angle = input.clock * 3 + index;
    projector.project(x + 18 * Math.cos(angle), y, z + 18 * Math.sin(angle));
    electron = { x: projector.x, y: projector.y, radius: 2.2 * projector.scale };
  }
  projector.project(x, y + BALLOON_LIFT, z);
  return { ...mark, electron, anchorX: projector.x, anchorY: projector.y, depth: projector.depth };
}

function labelFor(input: FrameInput, project: ProjectSlot, rise: number, opacity: number): ProjectLabel {
  const { projector } = input;
  projector.project(project.x, 0, project.z);
  const drop = 76 * project.scale * projector.perspective * projector.scale * projector.pitchSine + 10;
  return { repo: project.repo, sessionCount: project.sessions.length, x: projector.x, y: projector.y + drop, opacity: (0.3 + 0.7 * rise) * opacity };
}

function drawProject(state: FrameState, project: ProjectSlot): void {
  const { input, layers, ring } = state;
  const rise = input.roster.projectRise(project.repo, input.now);
  if (rise <= 0) return;
  const entries = project.sessions.flatMap((session) => state.entryById.get(session.id) ?? []);
  const opacity = projectOpacity(entries, input);
  const spin = input.clock * (project.working ? SPIN_WORKING : SPIN_CALM) + project.index;
  const shape = state.list.begin({ kind: 'lines', color: STATUS_COLORS[project.status], width: 1.3, alpha: SHAPE_ALPHA * opacity, glow: true });
  drawShape(shape, input, shapeGeometry(shapeFor(project.repo, input.rules)), project, rise, spin);
  if (rise < 1) drawRing(layers.scan, input.projector, project.x, 200 * rise * project.scale, project.z, 88 * project.scale, 32);
  state.overlay.labels.push(labelFor(input, project, rise, opacity));
  if (ring) drawLink(layers, input, project);
  drawRing(layers.orbits, input.projector, project.x, ORBIT_HEIGHT * rise * project.scale, project.z, ORBIT_RADIUS * project.scale, 48);
  entries.forEach((entry, index) => {
    state.overlay.turnAngles.set(entry.session.id, ring ? project.outward : null);
    if (input.roster.opacity(entry, input.now) > 0) state.marks.push(markSession(input, project, entry, index, rise));
  });
}

function drawSessions(input: FrameInput, list: DrawList, marks: readonly SessionMark[], overlay: FrameOverlay): void {
  const anchors = marks.map((mark) => ({
    id: mark.id,
    x: mark.anchorX,
    y: mark.anchorY,
    depth: mark.depth,
    height: input.balloonHeights.get(mark.id) ?? DEFAULT_BALLOON_HEIGHT,
    selected: mark.id === input.selectedId,
  }));
  const placements = layoutBalloons(anchors, { width: input.viewport.width, top: input.headerBottom });
  const leaderLines = new Map(STATUS_ORDER.map((status) => [status, list.begin({ kind: 'lines', color: STATUS_COLORS[status], width: 1, alpha: 0.75 })]));
  const leaderEnds = new Map(STATUS_ORDER.map((status) => [status, list.begin({ kind: 'rings', color: STATUS_COLORS[status], width: 1, alpha: 0.75 })]));
  const phase = fraction(input.clock / PULSE_SECONDS);
  const pulses = input.reducedMotion ? null : list.begin({ kind: 'rings', color: STATUS_COLORS.waiting, width: 1.5, alpha: 1 - phase });
  const satellites = new Map(STATUS_ORDER.map((status) => [status, list.begin({ kind: 'dots', color: STATUS_COLORS[status], width: 0, alpha: 1 })]));
  const electrons = list.begin({ kind: 'dots', color: STRUCTURE_COLOR, width: 0, alpha: 0.9 });
  marks.forEach((mark, index) => {
    const placement = placements[index];
    if (!placement) return;
    leaderLines.get(mark.status)?.line(mark.x, mark.y, placement.x, placement.bottom);
    leaderEnds.get(mark.status)?.circle(placement.x, placement.bottom, 2.5);
    if (mark.status === 'waiting') pulses?.circle(mark.x, mark.y, (6 + 18 * phase) * input.projector.scale);
    satellites.get(mark.status)?.circle(mark.x, mark.y, mark.radius);
    if (mark.electron) electrons.circle(mark.electron.x, mark.electron.y, mark.electron.radius);
    overlay.balloons.push({ ...placement, opacity: placement.opacity * Math.min(1, mark.opacity * 1.5) });
  });
}

/** Fills `list` with this frame's drawing and returns where the HTML labels and balloons go. */
export function buildFrame(input: FrameInput, list: DrawList): FrameOverlay {
  list.clear();
  const overlay: FrameOverlay = { labels: [], balloons: [], turnAngles: new Map() };
  const layout = layoutProjects(input.entries.map((entry) => entry.session));
  const empty = input.entries.length === 0;
  drawFloor(list.begin({ kind: 'lines', color: STRUCTURE_COLOR, width: 1, alpha: empty ? 0.1 : 0.16 }), input.projector);
  const layers: FrameLayers = {
    orbits: list.begin({ kind: 'lines', color: STRUCTURE_COLOR, width: 1, alpha: 0.4, dash: 'orbit' }),
    links: list.begin({ kind: 'lines', color: STRUCTURE_COLOR, width: 1, alpha: 0.45, dash: 'link' }),
    packets: list.begin({ kind: 'dots', color: STRUCTURE_COLOR, width: 0, alpha: 0.9 }),
    scan: list.begin({ kind: 'lines', color: STRUCTURE_COLOR, width: 2, alpha: 0.9 }),
  };
  if (layout.hasCore) drawCore(list.begin({ kind: 'lines', color: STRUCTURE_COLOR, width: 1.3, alpha: empty ? 0.5 : 0.9, glow: true }), input.projector, input.clock);
  const state: FrameState = {
    input,
    list,
    layers,
    overlay,
    marks: [],
    entryById: new Map(input.entries.map((entry) => [entry.session.id, entry])),
    ring: layout.projects.length > 1,
  };
  for (const project of layout.projects) drawProject(state, project);
  drawSessions(input, list, state.marks, overlay);
  return overlay;
}
