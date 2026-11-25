import type { FloorSpot } from "#/shared/agent.ts";

interface Position {
  readonly left: number;
  readonly top: number;
}

type Layout = Readonly<Record<string, Position>>;

interface Machine {
  readonly key: string;
  readonly spot: FloorSpot;
}

interface Mark {
  readonly height: number;
  readonly left: number;
  readonly top: number;
  readonly width: number;
}

interface Grip {
  readonly key: string;
  readonly origin: Position;
  readonly startX: number;
  readonly startY: number;
}

const COLUMNS = 3;
const DRAG_THRESHOLD = 5;
const EMPTY_LAYOUT: Layout = {};
const FIRST_SLOT = 0;
const FLOOR_EDGE = 0;
const GRID_STEP = 20;
const MACHINE_HEIGHT = 160;
const MACHINE_WIDTH = 280;
const MAP_INSET = 3;
const MAP_SCALE = 0.15;
const NEXT_SLOT = 1;
const ORIGIN = 40;
const PITCH_LEFT = 320;
const PITCH_TOP = 220;

const snapped = (value: number): number =>
  Math.max(FLOOR_EDGE, Math.round(value / GRID_STEP) * GRID_STEP);

const movedBy = (from: Position, deltaLeft: number, deltaTop: number): Position => ({
  left: snapped(from.left + deltaLeft),
  top: snapped(from.top + deltaTop),
});

const pulledPast = (grip: Grip, clientX: number, clientY: number): boolean =>
  Math.abs(clientX - grip.startX) + Math.abs(clientY - grip.startY) >= DRAG_THRESHOLD;

const draggedTo = (grip: Grip, clientX: number, clientY: number): Position =>
  movedBy(grip.origin, clientX - grip.startX, clientY - grip.startY);

const slotAt = (index: number): Position => ({
  left: ORIGIN + (index % COLUMNS) * PITCH_LEFT,
  top: ORIGIN + Math.floor(index / COLUMNS) * PITCH_TOP,
});

const firstSlots = (count: number): readonly Position[] =>
  [...Array.from({ length: count }).keys()].map((index) => slotAt(index));

const overlaps = (one: Position, other: Position): boolean =>
  Math.abs(one.left - other.left) < MACHINE_WIDTH && Math.abs(one.top - other.top) < MACHINE_HEIGHT;

const samePosition = (one: Position, other: Position): boolean =>
  one.left === other.left && one.top === other.top;

const freeSlotFrom = (index: number, taken: readonly Position[]): Position => {
  const slot = slotAt(index);
  if (taken.some((position) => overlaps(position, slot))) {
    return freeSlotFrom(index + NEXT_SLOT, taken);
  }
  return slot;
};

const placedAll = (layout: Layout, keys: readonly string[]): Layout => {
  if (keys.every((key) => Object.hasOwn(layout, key))) {
    return layout;
  }
  const placed = new Map(Object.entries(layout));
  const taken = keys.flatMap((key) => placed.get(key) ?? []);
  for (const key of keys.filter((candidate) => !placed.has(candidate))) {
    const slot = freeSlotFrom(FIRST_SLOT, taken);
    placed.set(key, slot);
    taken.push(slot);
  }
  return Object.fromEntries(placed);
};

const daemonPosition = (machine: Machine): readonly Position[] => {
  if (!machine.spot.placed) {
    return [];
  }
  return [{ left: machine.spot.left, top: machine.spot.top }];
};

const daemonLayout = (machines: readonly Machine[]): Layout =>
  Object.fromEntries(
    machines.flatMap((machine) =>
      daemonPosition(machine).map((position): [string, Position] => [machine.key, position]),
    ),
  );

const shownLayout = (machines: readonly Machine[], held: Layout): Layout => {
  const keys = machines.map((machine) => machine.key);
  const present = Object.entries(held).filter(([key]) => keys.includes(key));
  return placedAll({ ...daemonLayout(machines), ...Object.fromEntries(present) }, keys);
};

const unplacedKeys = (machines: readonly Machine[]): readonly string[] =>
  machines.filter((machine) => !machine.spot.placed).map((machine) => machine.key);

const stillHeld = (held: Layout, machines: readonly Machine[]): Layout => {
  const agreed = daemonLayout(machines);
  const keys = new Set(machines.map((machine) => machine.key));
  const waiting = Object.entries(held).filter(([key, position]) => {
    const settled = agreed[key];
    return keys.has(key) && !(settled && samePosition(settled, position));
  });
  if (waiting.length === Object.keys(held).length) {
    return held;
  }
  return Object.fromEntries(waiting);
};

const positionOf = (layout: Layout, key: string): Position => layout[key] ?? slotAt(FIRST_SLOT);

const withPosition = (layout: Layout, key: string, position: Position): Layout => ({
  ...layout,
  [key]: position,
});

const withoutPosition = (layout: Layout, key: string): Layout =>
  Object.fromEntries(Object.entries(layout).filter(([held]) => held !== key));

const markOf = (position: Position): Mark => ({
  height: Math.round(MACHINE_HEIGHT * MAP_SCALE),
  left: Math.round(MAP_INSET + position.left * MAP_SCALE),
  top: Math.round(position.top * MAP_SCALE),
  width: Math.round(MACHINE_WIDTH * MAP_SCALE),
});

const extentOf = (positions: readonly Position[]): Position => ({
  left:
    Math.max(FLOOR_EDGE, ...positions.map((position) => position.left)) + MACHINE_WIDTH + ORIGIN,
  top: Math.max(FLOOR_EDGE, ...positions.map((position) => position.top)) + MACHINE_HEIGHT + ORIGIN,
});

export {
  EMPTY_LAYOUT,
  GRID_STEP,
  MACHINE_HEIGHT,
  MACHINE_WIDTH,
  draggedTo,
  extentOf,
  firstSlots,
  markOf,
  movedBy,
  positionOf,
  pulledPast,
  shownLayout,
  stillHeld,
  unplacedKeys,
  withPosition,
  withoutPosition,
};
export type { Grip, Layout, Machine, Mark, Position };
