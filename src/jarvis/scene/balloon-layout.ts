export interface BalloonAnchor {
  id: string;
  /** Screen point above the satellite where the balloon's bottom edge wants to sit. */
  x: number;
  y: number;
  depth: number;
  height: number;
  selected: boolean;
}

export interface BalloonPlacement {
  id: string;
  /** Horizontal centre and bottom edge, in CSS pixels. */
  x: number;
  bottom: number;
  opacity: number;
  zIndex: number;
}

export interface BalloonBounds {
  width: number;
  /** Lowest y of the header; balloons stay below it. */
  top: number;
}

export const BALLOON_WIDTH = 272;
const EDGE_MARGIN = 12;
const COLLISION_GAP = 8;
const STACK_GAP = 10;
const MAX_PUSHES = 8;
const BACK_DEPTH = 60;
const BACK_OPACITY = 0.72;
const SELECTED_Z_INDEX = 2000;
const BASE_Z_INDEX = 1000;

interface PlacedBox {
  x: number;
  top: number;
  bottom: number;
}

function clamp(value: number, min: number, max: number): number {
  return value < min ? min : value > max ? max : value;
}

function findCollision(placed: readonly PlacedBox[], x: number, bottom: number, height: number): PlacedBox | undefined {
  return placed.find(
    (box) => Math.abs(box.x - x) < BALLOON_WIDTH + COLLISION_GAP && bottom > box.top - COLLISION_GAP && bottom - height < box.bottom + COLLISION_GAP,
  );
}

/** Nearest first: each balloon that hits one already placed is pushed up above it. */
export function layoutBalloons(anchors: readonly BalloonAnchor[], bounds: BalloonBounds): BalloonPlacement[] {
  const nearestFirst = [...anchors].sort((first, second) => first.depth - second.depth);
  const placed: PlacedBox[] = [];
  const placements = new Map<BalloonAnchor, BalloonPlacement>();
  nearestFirst.forEach((anchor, order) => {
    const x = clamp(anchor.x, BALLOON_WIDTH / 2 + EDGE_MARGIN, bounds.width - BALLOON_WIDTH / 2 - EDGE_MARGIN);
    let bottom = anchor.y;
    for (let push = 0; push < MAX_PUSHES; push += 1) {
      const collision = findCollision(placed, x, bottom, anchor.height);
      if (!collision) break;
      bottom = collision.top - STACK_GAP;
    }
    if (bottom - anchor.height < bounds.top) bottom = bounds.top + anchor.height;
    placed.push({ x, top: bottom - anchor.height, bottom });
    placements.set(anchor, {
      id: anchor.id,
      x,
      bottom,
      opacity: anchor.selected || anchor.depth <= BACK_DEPTH ? 1 : BACK_OPACITY,
      zIndex: anchor.selected ? SELECTED_Z_INDEX : BASE_Z_INDEX - order,
    });
  });
  return anchors.flatMap((anchor) => placements.get(anchor) ?? []);
}
