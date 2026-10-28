import { describe, expect, it } from 'vitest';
import { Projector } from './projection';

const DESIGN_VIEWPORT = { width: 1200, height: 760 };
const FRONT = { yaw: 0, pitch: 0, zoom: 1 };

function projected(projector: Projector, x: number, y: number, z: number) {
  projector.project(x, y, z);
  return { x: projector.x, y: projector.y, depth: projector.depth, perspective: projector.perspective };
}

describe('Projector', () => {
  it('puts the scene centre at the middle of the width and 60% of the height, whatever the angle', () => {
    const projector = new Projector();
    for (const pose of [FRONT, { yaw: 1.3, pitch: 0.46, zoom: 1.4 }]) {
      projector.configure(DESIGN_VIEWPORT, pose);
      const centre = projected(projector, 0, 60, 0);
      expect(centre.x).toBeCloseTo(600);
      expect(centre.y).toBeCloseTo(456);
    }
  });

  it('applies the 1000 focal length at a 1350 distance', () => {
    const projector = new Projector();
    projector.configure(DESIGN_VIEWPORT, FRONT);
    expect(projected(projector, 100, 60, 0).x).toBeCloseTo(674.074, 2);
    const far = projected(projector, 100, 60, 100);
    expect(far.depth).toBeCloseTo(100);
    expect(far.x).toBeCloseTo(668.966, 2);
  });

  it('tilts far points upward when the camera pitches down', () => {
    const projector = new Projector();
    projector.configure(DESIGN_VIEWPORT, { yaw: 0, pitch: Math.PI / 2, zoom: 1 });
    expect(projected(projector, 0, 60, 100).y).toBeCloseTo(381.926, 2);
  });

  it('turns the world with the yaw', () => {
    const projector = new Projector();
    projector.configure(DESIGN_VIEWPORT, { yaw: Math.PI / 2, pitch: 0, zoom: 1 });
    const turned = projected(projector, 100, 60, 0);
    expect(turned.x).toBeCloseTo(600);
    expect(turned.depth).toBeCloseTo(100);
  });

  it('scales with min(width / 1200, height / 760) times the zoom', () => {
    const projector = new Projector();
    projector.configure({ width: 600, height: 760 }, { yaw: 0, pitch: 0, zoom: 2 });
    expect(projector.scale).toBeCloseTo(1);
    expect(projected(projector, 100, 60, 0).x).toBeCloseTo(300 + 74.074, 2);
  });
});
