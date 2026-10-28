import type { ShapeChoice, ShapeKind } from './shape-rules';

/** Wireframe in world units, local to the shape's base (y up, floor at y = 0). */
export interface ShapeGeometry {
  /** Six floats per segment: ax, ay, az, bx, by, bz. */
  readonly segments: Float32Array;
  /** 1 where the segment always faces the camera (the eye's almond and pupil). */
  readonly billboard: Uint8Array;
  readonly count: number;
}

type Vertex = readonly [number, number, number];

class GeometryBuilder {
  private readonly coordinates: number[] = [];
  private readonly facing: number[] = [];

  segment(from: Vertex, to: Vertex, billboard = false): void {
    this.coordinates.push(from[0], from[1], from[2], to[0], to[1], to[2]);
    this.facing.push(billboard ? 1 : 0);
  }

  /** Closed polyline: each vertex joins the next and the last joins the first. */
  loop(vertices: readonly Vertex[], billboard = false): void {
    let previous = vertices.at(-1);
    for (const vertex of vertices) {
      if (previous) this.segment(previous, vertex, billboard);
      previous = vertex;
    }
  }

  box(centerX: number, centerZ: number, halfX: number, halfZ: number, bottom: number, top: number): void {
    const corners = [
      [centerX - halfX, centerZ - halfZ],
      [centerX + halfX, centerZ - halfZ],
      [centerX + halfX, centerZ + halfZ],
      [centerX - halfX, centerZ + halfZ],
    ] as const;
    this.loop(corners.map(([x, z]): Vertex => [x, bottom, z]));
    this.loop(corners.map(([x, z]): Vertex => [x, top, z]));
    for (const [x, z] of corners) this.segment([x, bottom, z], [x, top, z]);
  }

  build(): ShapeGeometry {
    return { segments: Float32Array.from(this.coordinates), billboard: Uint8Array.from(this.facing), count: this.facing.length };
  }
}

function ringPoint(radius: number, y: number, sides: number, index: number, rotation = 0): Vertex {
  const angle = (index / sides) * Math.PI * 2 + rotation;
  return [radius * Math.cos(angle), y, radius * Math.sin(angle)];
}

function ring(radius: number, y: number, sides: number, rotation = 0): Vertex[] {
  return Array.from({ length: sides }, (_, index) => ringPoint(radius, y, sides, index, rotation));
}

function buildTower(shape: GeometryBuilder): void {
  const apex: Vertex = [0, 198, 0];
  shape.loop(ring(64, 0, 6));
  shape.loop(ring(36, 100, 6, Math.PI / 6));
  shape.loop(ring(18, 150, 6, Math.PI / 6));
  for (let side = 0; side < 6; side += 1) {
    const middle = ringPoint(36, 100, 6, side, Math.PI / 6);
    shape.segment(ringPoint(64, 0, 6, side), middle);
    shape.segment(ringPoint(64, 0, 6, side + 1), middle);
    shape.segment(middle, apex);
  }
}

/** One edge of the almond: the upper lid with `lid` = 1, the lower with -1. */
function almondPoint(index: number, steps: number, lid: 1 | -1): Vertex {
  const angle = (index / steps) * Math.PI;
  return [-80 * Math.cos(angle), 150 + lid * 34 * Math.sin(angle), 0];
}

function buildEye(shape: GeometryBuilder): void {
  shape.loop(ring(24, 0, 4, Math.PI / 4));
  shape.loop(ring(12, 104, 4, Math.PI / 4));
  for (let corner = 0; corner < 4; corner += 1) shape.segment(ringPoint(24, 0, 4, corner, Math.PI / 4), ringPoint(12, 104, 4, corner, Math.PI / 4));
  shape.segment([0, 104, 0], [0, 116, 0]);
  shape.loop(ring(96, 150, 24));
  const steps = 14;
  for (let index = 0; index < steps; index += 1) {
    shape.segment(almondPoint(index, steps, 1), almondPoint(index + 1, steps, 1), true);
    shape.segment(almondPoint(index, steps, -1), almondPoint(index + 1, steps, -1), true);
  }
  const pupil = Array.from({ length: 12 }, (_, index): Vertex => {
    const angle = (index / 12) * Math.PI * 2;
    return [9 * Math.sin(angle), 150 + 27 * Math.cos(angle), 0];
  });
  shape.loop(pupil, true);
}

const FORTRESS_CORNERS = [
  [-1, -1],
  [1, -1],
  [1, 1],
  [-1, 1],
] as const;

function buildFortress(shape: GeometryBuilder): void {
  shape.box(0, 0, 58, 58, 0, 72);
  for (const [cornerX, cornerZ] of FORTRESS_CORNERS) shape.box(cornerX * 58, cornerZ * 58, 15, 15, 72, 106);
  shape.segment([-16, 0, -58], [-16, 36, -58]);
  shape.segment([-16, 36, -58], [16, 36, -58]);
  shape.segment([16, 36, -58], [16, 0, -58]);
}

function buildOrb(shape: GeometryBuilder): void {
  const center = 112;
  const radius = 78;
  shape.loop(ring(radius, center, 28));
  shape.loop(ring(63, center + 46, 22));
  shape.loop(ring(63, center - 46, 22));
  const meridian = (alongX: boolean) =>
    Array.from({ length: 28 }, (_, index): Vertex => {
      const angle = (index / 28) * Math.PI * 2;
      const across = radius * Math.cos(angle);
      return [alongX ? across : 0, center + radius * Math.sin(angle), alongX ? 0 : across];
    });
  shape.loop(meridian(true));
  shape.loop(meridian(false));
  shape.loop(ring(30, 0, 18));
  for (let leg = 0; leg < 3; leg += 1) shape.segment(ringPoint(30, 0, 3, leg), ringPoint(16, 30, 3, leg));
}

interface CrystalProportions {
  sides: number;
  waistRadius: number;
  waistY: number;
  apexY: number;
  footY: number;
}

const BASE_CRYSTAL: CrystalProportions = { sides: 4, waistRadius: 58, waistY: 112, apexY: 200, footY: 30 };

const CRYSTALS: readonly CrystalProportions[] = [
  BASE_CRYSTAL,
  { sides: 6, waistRadius: 50, waistY: 118, apexY: 212, footY: 34 },
  { sides: 5, waistRadius: 62, waistY: 104, apexY: 190, footY: 26 },
];

function buildCrystal(shape: GeometryBuilder, variant: number): void {
  const proportions = CRYSTALS[variant] ?? BASE_CRYSTAL;
  const apex: Vertex = [0, proportions.apexY, 0];
  const foot: Vertex = [0, proportions.footY, 0];
  const waist = ring(proportions.waistRadius, proportions.waistY, proportions.sides);
  shape.loop(waist);
  for (const vertex of waist) {
    shape.segment(vertex, apex);
    shape.segment(vertex, foot);
  }
  shape.segment([0, 0, 0], foot);
  shape.loop(ring(30, 0, 16));
}

const BUILDERS: Record<ShapeKind, (shape: GeometryBuilder, variant: number) => void> = {
  tower: buildTower,
  eye: buildEye,
  fortress: buildFortress,
  orb: buildOrb,
  crystal: buildCrystal,
};

const cache = new Map<string, ShapeGeometry>();

export function shapeGeometry(choice: ShapeChoice): ShapeGeometry {
  const cacheKey = `${choice.kind}:${choice.variant}`;
  const cached = cache.get(cacheKey);
  if (cached) return cached;
  const shape = new GeometryBuilder();
  BUILDERS[choice.kind](shape, choice.variant);
  const geometry = shape.build();
  cache.set(cacheKey, geometry);
  return geometry;
}
