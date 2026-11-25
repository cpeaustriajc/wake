import { Array, Option, Record } from "effect";

import type { Position } from "#/shared/agent.ts";

type Layout = Record.ReadonlyRecord<string, Position>;

interface Machine {
  readonly key: string;
  readonly spot: Position | null;
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
const GRID_STEP = 20;
const MACHINE_HEIGHT = 160;
const MACHINE_WIDTH = 280;
const MAP_INSET = 3;
const MAP_SCALE = 0.15;
const ORIGIN = 40;
const PITCH_X = 320;
const PITCH_Y = 220;

const snapped = (value: number): number =>
  Math.max(0, Math.round(value / GRID_STEP) * GRID_STEP);

interface Move {
  readonly deltaX: number;
  readonly deltaY: number;
  readonly from: Position;
}

interface Pull {
  readonly clientX: number;
  readonly clientY: number;
  readonly grip: Grip;
}

const movedBy = ({ deltaX, deltaY, from }: Move): Position => ({
  x: snapped(from.x + deltaX),
  y: snapped(from.y + deltaY),
});

const stepsToPixels = (steps: number): number => steps * GRID_STEP;

const pulledPast = ({ clientX, clientY, grip }: Pull): boolean =>
  Math.abs(clientX - grip.startX) + Math.abs(clientY - grip.startY) >=
  DRAG_THRESHOLD;

const draggedTo = ({ clientX, clientY, grip }: Pull): Position =>
  movedBy({
    deltaX: clientX - grip.startX,
    deltaY: clientY - grip.startY,
    from: grip.origin,
  });

const slotAt = (index: number): Position => ({
  x: ORIGIN + (index % COLUMNS) * PITCH_X,
  y: ORIGIN + Math.floor(index / COLUMNS) * PITCH_Y,
});

const firstSlots = (count: number): readonly Position[] =>
  Array.makeBy(count, slotAt);

const overlaps = (one: Position, other: Position): boolean =>
  Math.abs(one.x - other.x) < MACHINE_WIDTH &&
  Math.abs(one.y - other.y) < MACHINE_HEIGHT;

const samePosition = (one: Position, other: Position): boolean =>
  one.x === other.x && one.y === other.y;

const freeSlotFrom = (index: number, taken: readonly Position[]): Position => {
  const slot = slotAt(index);
  return Array.some(taken, (position) => overlaps(position, slot))
    ? freeSlotFrom(index + 1, taken)
    : slot;
};

const positionsOf = (
  layout: Layout,
  keys: readonly string[]
): readonly Position[] =>
  Array.flatMap(keys, (key) => Option.toArray(Record.get(layout, key)));

interface Placing {
  readonly keys: readonly string[];
  readonly layout: Layout;
}

interface Showing {
  readonly held: Layout;
  readonly machines: readonly Machine[];
}

interface Lookup {
  readonly key: string;
  readonly layout: Layout;
}

const placedAll = ({ keys, layout }: Placing): Layout =>
  Array.reduce(
    Array.filter(keys, (key) => !Record.has(layout, key)),
    layout,
    (placed, key) =>
      Record.set(placed, key, freeSlotFrom(0, positionsOf(placed, keys)))
  );

const daemonLayout = (machines: readonly Machine[]): Layout =>
  Record.fromEntries(
    Array.flatMap(machines, ({ key, spot }): [string, Position][] =>
      spot === null ? [] : [[key, spot]]
    )
  );

const keysOf = (machines: readonly Machine[]): readonly string[] =>
  Array.map(machines, ({ key }) => key);

const shownLayout = ({ held, machines }: Showing): Layout => {
  const keys = keysOf(machines);
  const present = Record.filter(held, (_position, key) =>
    Array.some(keys, (known) => known === key)
  );
  return placedAll({
    keys,
    layout: { ...daemonLayout(machines), ...present },
  });
};

const unplacedKeys = (machines: readonly Machine[]): readonly string[] =>
  keysOf(Array.filter(machines, ({ spot }) => spot === null));

const stillHeld = ({ held, machines }: Showing): Layout => {
  const agreed = daemonLayout(machines);
  const keys = keysOf(machines);
  return Record.filter(
    held,
    (position, key) =>
      Array.some(keys, (known) => known === key) &&
      !Option.exists(Record.get(agreed, key), (settled) =>
        samePosition(settled, position)
      )
  );
};

const positionOf = ({ key, layout }: Lookup): Position =>
  Option.getOrElse(Record.get(layout, key), () => slotAt(0));

const markOf = (position: Position): Mark => ({
  height: Math.round(MACHINE_HEIGHT * MAP_SCALE),
  left: Math.round(MAP_INSET + position.x * MAP_SCALE),
  top: Math.round(position.y * MAP_SCALE),
  width: Math.round(MACHINE_WIDTH * MAP_SCALE),
});

const extentOf = (positions: readonly Position[]): Position => ({
  x:
    Math.max(0, ...Array.map(positions, ({ x }) => x)) + MACHINE_WIDTH + ORIGIN,
  y:
    Math.max(0, ...Array.map(positions, ({ y }) => y)) +
    MACHINE_HEIGHT +
    ORIGIN,
});

export {
  EMPTY_LAYOUT,
  GRID_STEP,
  MACHINE_HEIGHT,
  MACHINE_WIDTH,
  PITCH_X,
  PITCH_Y,
  draggedTo,
  extentOf,
  firstSlots,
  markOf,
  movedBy,
  placedAll,
  positionOf,
  pulledPast,
  shownLayout,
  slotAt,
  snapped,
  stepsToPixels,
  stillHeld,
  unplacedKeys,
};
export type { Grip, Layout, Machine, Mark, Pull };
