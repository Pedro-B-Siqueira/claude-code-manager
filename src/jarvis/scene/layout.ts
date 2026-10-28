import { STATUS_PRIORITY, type JarvisSession, type JarvisStatus } from '../data/jarvis-session';

export interface ProjectSlot {
  repo: string;
  /** Position in the ring, also used to offset each shape's spin. */
  index: number;
  x: number;
  z: number;
  /** Direction from the scene centre to the project; sessions fan out around it. */
  outward: number;
  scale: number;
  sessions: JarvisSession[];
  status: JarvisStatus;
  working: boolean;
}

export interface SceneLayout {
  projects: ProjectSlot[];
  /** The core shows with no project or with two or more; a lone project takes the centre. */
  hasCore: boolean;
}

export const ORBIT_RADIUS = 112;
export const ORBIT_HEIGHT = 150;
const FAN_STEP = 0.95;
const SOLO_SCALE = 1.2;

function ringRadius(projectCount: number): number {
  if (projectCount <= 1) return 0;
  return projectCount === 2 ? 210 : 260;
}

function dominantStatus(sessions: readonly JarvisSession[]): JarvisStatus {
  return sessions.reduce<JarvisStatus>((top, session) => (STATUS_PRIORITY[session.status] > STATUS_PRIORITY[top] ? session.status : top), 'idle');
}

function groupByRepo(sessions: readonly JarvisSession[]): Map<string, JarvisSession[]> {
  const groups = new Map<string, JarvisSession[]>();
  for (const session of sessions) {
    const group = groups.get(session.repo);
    if (group) group.push(session);
    else groups.set(session.repo, [session]);
  }
  return groups;
}

export function layoutProjects(sessions: readonly JarvisSession[]): SceneLayout {
  const groups = [...groupByRepo(sessions)];
  const count = groups.length;
  const radius = ringRadius(count);
  const projects = groups.map(([repo, members], index): ProjectSlot => {
    const angle = count > 1 ? -Math.PI / 2 + (index * 2 * Math.PI) / count : 0;
    return {
      repo,
      index,
      x: radius * Math.cos(angle),
      z: radius * Math.sin(angle),
      outward: count > 1 ? angle : -Math.PI / 2,
      scale: count === 1 ? SOLO_SCALE : 1,
      sessions: members,
      status: dominantStatus(members),
      working: members.some((member) => member.status === 'working'),
    };
  });
  return { projects, hasCore: count !== 1 };
}

/** Horizontal offset of a session's satellite from its project's base. */
export function satelliteOffset(project: ProjectSlot, index: number, count: number): { x: number; z: number } {
  const angle = project.outward + (index - (count - 1) / 2) * FAN_STEP;
  const radius = ORBIT_RADIUS * project.scale;
  return { x: radius * Math.cos(angle), z: radius * Math.sin(angle) };
}
