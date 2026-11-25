import { Array, Option, Record } from "effect";
import * as fc from "fast-check";
import { describe, expect, it } from "vitest";

import {
  GRID_STEP,
  MACHINE_HEIGHT,
  MACHINE_WIDTH,
  PITCH_X,
  PITCH_Y,
  shownLayout,
  snapped,
  stillHeld,
  unplacedKeys,
} from "#/renderer/floor/floor-model.ts";
import type { Machine } from "#/renderer/floor/floor-model.ts";
import type { Position } from "#/shared/agent.ts";

const ORIGIN = 40;

const position = fc.record({
  x: fc.integer({ max: 2000, min: 0 }).map(snapped),
  y: fc.integer({ max: 2000, min: 0 }).map(snapped),
});

const machines = fc
  .array(fc.option(position, { nil: null }), { maxLength: 12 })
  .map((spots) =>
    Array.map(spots, (spot, index): Machine => ({ key: `a${index}`, spot }))
  );

const placedMachine = fc.record({
  key: fc.constant("a0"),
  spot: position,
});

const overlap = (one: Position, other: Position): boolean =>
  Math.abs(one.x - other.x) < MACHINE_WIDTH &&
  Math.abs(one.y - other.y) < MACHINE_HEIGHT;

const placedSpots = (fleet: readonly Machine[]): readonly Position[] =>
  Array.flatMap(fleet, ({ spot }) => (spot === null ? [] : [spot]));

const assignedSlots = (fleet: readonly Machine[]): readonly Position[] => {
  const layout = shownLayout({ held: {}, machines: fleet });
  return Array.flatMap(unplacedKeys(fleet), (key) =>
    Option.toArray(Record.get(layout, key))
  );
};

const crowded = (slots: readonly Position[], taken: readonly Position[]) =>
  Array.filter(
    slots,
    (slot, index) =>
      Array.some(taken, (other) => overlap(other, slot)) ||
      Array.some(slots, (other, at) => at !== index && overlap(other, slot))
  );

const offPitch = (slots: readonly Position[]) =>
  Array.filter(
    slots,
    ({ x, y }) =>
      (x - ORIGIN) % PITCH_X !== 0 ||
      (y - ORIGIN) % PITCH_Y !== 0 ||
      Math.min(x, y) < 0
  );

describe("the floor model", () => {
  it("puts every unplaced agent on a free slot of the 320 by 220 pitch", () => {
    fc.assert(
      fc.property(machines, (fleet) => {
        const slots = assignedSlots(fleet);
        expect(slots).toHaveLength(unplacedKeys(fleet).length);
        expect(offPitch(slots)).toStrictEqual([]);
        expect(crowded(slots, placedSpots(fleet))).toStrictEqual([]);
      })
    );
  });

  it("shows a placed agent where the daemon keeps it", () => {
    fc.assert(
      fc.property(machines, (fleet) => {
        const layout = shownLayout({ held: {}, machines: fleet });
        expect(
          Array.flatMap(fleet, ({ key, spot }) =>
            spot === null ? [] : Option.toArray(Record.get(layout, key))
          )
        ).toStrictEqual(placedSpots(fleet));
      })
    );
  });

  it("shows a held position in place of the daemon's", () => {
    fc.assert(
      fc.property(placedMachine, position, (machine, held) => {
        const layout = shownLayout({
          held: { [machine.key]: held },
          machines: [machine],
        });
        expect(layout).toStrictEqual({ [machine.key]: held });
      })
    );
  });

  it("drops a held position when the daemon agrees and keeps it otherwise", () => {
    fc.assert(
      fc.property(placedMachine, position, (machine, held) => {
        const kept = stillHeld({
          held: { [machine.key]: held },
          machines: [machine],
        });
        const agreed = machine.spot.x === held.x && machine.spot.y === held.y;
        expect(Record.keys(kept)).toStrictEqual(agreed ? [] : [machine.key]);
      })
    );
  });

  it("drops a held position when its agent left the fleet", () => {
    fc.assert(
      fc.property(position, (held) => {
        expect(stillHeld({ held: { gone: held }, machines: [] })).toStrictEqual(
          {}
        );
      })
    );
  });

  it("snaps to the grid, never below zero, by less than one step, and once is enough", () => {
    fc.assert(
      fc.property(fc.integer({ max: 100_000, min: -100_000 }), (value) => {
        const once = snapped(value);
        expect(once).toBeGreaterThanOrEqual(0);
        expect(once % GRID_STEP).toBe(0);
        expect(snapped(once)).toBe(once);
        expect(Math.abs(once - Math.max(0, value))).toBeLessThan(GRID_STEP);
      })
    );
  });
});
