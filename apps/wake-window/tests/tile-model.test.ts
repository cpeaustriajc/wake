import { Array, Option, String } from "effect";
import * as fc from "fast-check";
import { describe, expect, it } from "vitest";

import { gapLabel, tilesOf, whenOf } from "#/renderer/floor/tile-model.ts";
import type { Agent } from "#/shared/agent.ts";

const UNITS = ["just now", "min ago", "h ago", "d ago"] as const;
const UNIT_WEIGHT = 1_000_000;
const NOON = Date.UTC(2026, 10, 25, 12, 0, 0);

const leading = (label: string): number => {
  const [first] = String.split(label, " ");
  const count = Number(first);
  return Number.isNaN(count) ? 0 : count;
};

const rank = (label: string): number =>
  Option.getOrElse(
    Array.findFirstIndex(UNITS, (unit) => String.endsWith(unit)(label)),
    () => 0
  ) *
    UNIT_WEIGHT +
  leading(label);

const agent = (overrides: Partial<Agent>): Agent => ({
  branch: null,
  folder: "/work/one",
  id: 1,
  location: "local",
  position: null,
  progress: { state: "missing" },
  started_at: 1700,
  ...overrides,
});

const tiles = tilesOf({
  agents: [
    agent({
      progress: {
        headline: "Parser done",
        phase: " Review ",
        state: "found",
        updated: "2026-11-25T11:00:00Z",
      },
    }),
    agent({
      branch: "claude/task",
      id: 7,
      location: "cloud",
      progress: { reason: "no route", state: "unreachable" },
    }),
    agent({ id: 3, position: { x: 40, y: 40 } }),
  ],
  now: NOON,
});

describe("the tile model", () => {
  it("labels a gap monotonically and never with a negative number", () => {
    fc.assert(
      fc.property(
        fc.integer({ max: 10_000_000, min: -1000 }),
        fc.integer({ max: 10_000_000, min: -1000 }),
        (one, other) => {
          const shorter = gapLabel(Math.min(one, other));
          const longer = gapLabel(Math.max(one, other));
          expect(shorter).not.toContain("-");
          expect(rank(shorter)).toBeLessThanOrEqual(rank(longer));
        }
      )
    );
  });

  it("reads an RFC 3339 instant and leaves anything else blank", () => {
    expect(whenOf({ now: NOON, updated: "2026-11-25T11:47:00Z" })).toBe(
      "13 min ago"
    );
    expect(whenOf({ now: NOON, updated: "2026-11-25T14:00:00+02:00" })).toBe(
      "just now"
    );
    expect(whenOf({ now: NOON, updated: "yesterday" })).toBe("");
  });

  it("names local agents a and cloud tasks c", () => {
    expect(Array.map(tiles, ({ key }) => key)).toStrictEqual([
      "a1",
      "c7",
      "a3",
    ]);
  });

  it("lights the lamp from the phase and keeps the phase as written", () => {
    expect(
      Array.map(tiles, ({ lamp, status, when }) => [lamp, status, when])
    ).toStrictEqual([
      ["warning", " Review ", "1 h ago"],
      ["warning", "Unreachable", ""],
      ["idle", "No file", ""],
    ]);
  });

  it("carries the reason, the place and the daemon's position", () => {
    expect(
      Array.map(tiles, ({ note, place, spot }) => [note, place, spot])
    ).toStrictEqual([
      ["/work/one", "Local", null],
      ["no route", "Cloud, branch claude/task", null],
      ["/work/one", "Local", { x: 40, y: 40 }],
    ]);
  });
});
