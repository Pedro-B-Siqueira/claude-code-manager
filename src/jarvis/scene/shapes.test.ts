import { describe, expect, it } from 'vitest';
import { shapeGeometry, type ShapeGeometry } from './shapes';

interface Point {
  x: number;
  y: number;
  z: number;
  billboard: boolean;
}

function points(geometry: ShapeGeometry): Point[] {
  const result: Point[] = [];
  for (let index = 0; index < geometry.count; index += 1) {
    const offset = index * 6;
    const billboard = geometry.billboard[index] === 1;
    const at = (position: number) => geometry.segments[offset + position] ?? Number.NaN;
    result.push({ x: at(0), y: at(1), z: at(2), billboard });
    result.push({ x: at(3), y: at(4), z: at(5), billboard });
  }
  return result;
}

function highest(geometry: ShapeGeometry): number {
  return Math.max(...points(geometry).map((point) => point.y));
}

function radius(point: Point): number {
  return Math.hypot(point.x, point.z);
}

describe('shapeGeometry', () => {
  it('builds the tower with a hexagonal base, a narrower hexagon at mid height and the apex at y 198', () => {
    const tower = points(shapeGeometry({ kind: 'tower', variant: 0 }));
    expect(highest(shapeGeometry({ kind: 'tower', variant: 0 }))).toBeCloseTo(198);
    const base = tower.filter((point) => point.y === 0);
    expect(base.length).toBeGreaterThan(0);
    for (const point of base) expect(radius(point)).toBeCloseTo(64);
    expect(tower.some((point) => point.y === 100 && Math.abs(radius(point) - 36) < 0.01)).toBe(true);
    expect(tower.some((point) => point.y === 150)).toBe(true);
  });

  it('builds the eye with an almond and pupil that face the camera, over a horizontal halo', () => {
    const eye = points(shapeGeometry({ kind: 'eye', variant: 0 }));
    const facing = eye.filter((point) => point.billboard);
    expect(facing.length).toBeGreaterThan(0);
    for (const point of facing) {
      expect(point.z).toBe(0);
      expect(Math.abs(point.x)).toBeLessThanOrEqual(80.0001);
      expect(point.y).toBeGreaterThanOrEqual(116 - 0.0001);
      expect(point.y).toBeLessThanOrEqual(184 + 0.0001);
    }
    const halo = eye.filter((point) => !point.billboard && point.y === 150);
    expect(halo.length).toBeGreaterThan(0);
    for (const point of halo) expect(radius(point)).toBeCloseTo(96);
  });

  it('builds the fortress as a 116 × 72 × 116 keep with corner towers reaching y 106', () => {
    const fortress = shapeGeometry({ kind: 'fortress', variant: 0 });
    expect(highest(fortress)).toBeCloseTo(106);
    const keep = points(fortress).filter((point) => point.y <= 72);
    expect(Math.max(...keep.map((point) => Math.abs(point.x)))).toBeCloseTo(73);
    expect(points(fortress).some((point) => point.y === 72 && Math.abs(point.x) === 58 && Math.abs(point.z) === 58)).toBe(true);
  });

  it('builds the orb as a sphere of radius 78 centred at y 112 on a three-legged stand', () => {
    const orb = points(shapeGeometry({ kind: 'orb', variant: 0 }));
    const sphere = orb.filter((point) => point.y > 30);
    for (const point of sphere) expect(Math.hypot(point.x, point.y - 112, point.z)).toBeCloseTo(78, 0);
    expect(Math.max(...orb.map((point) => point.y))).toBeCloseTo(190);
  });

  it('builds the base crystal as a stretched octahedron on a mast', () => {
    const crystal = shapeGeometry({ kind: 'crystal', variant: 0 });
    expect(highest(crystal)).toBeCloseTo(200);
    expect(points(crystal).some((point) => point.x === 0 && point.y === 0 && point.z === 0)).toBe(true);
  });

  it('varies the crystal by variant', () => {
    const base = shapeGeometry({ kind: 'crystal', variant: 0 });
    const others = [1, 2].map((variant) => shapeGeometry({ kind: 'crystal', variant }));
    for (const other of others) expect(other.segments).not.toEqual(base.segments);
  });

  it('caches each geometry so frames never rebuild it', () => {
    expect(shapeGeometry({ kind: 'orb', variant: 0 })).toBe(shapeGeometry({ kind: 'orb', variant: 0 }));
    expect(shapeGeometry({ kind: 'crystal', variant: 1 })).not.toBe(shapeGeometry({ kind: 'crystal', variant: 2 }));
  });
});
