import { Array, Option, Record } from "effect";

import {
  draggedTo,
  movedBy,
  positionOf,
  pulledPast,
  shownLayout,
  stepsToPixels,
  stillHeld,
  unplacedKeys,
} from "#/renderer/floor/floor-model.ts";
import type { Grip, Layout } from "#/renderer/floor/floor-model.ts";
import { tilesOf } from "#/renderer/floor/tile-model.ts";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import type { Agent, Placement } from "#/shared/agent.ts";
import type { FleetReading, PlaceOutcome } from "#/shared/daemon-answer.ts";

type Trouble =
  | {
      readonly kind: "daemon_down";
      readonly reason: string;
      readonly socket: string;
    }
  | { readonly kind: "none" }
  | { readonly kind: "unreadable"; readonly reason: string };

interface FleetSnapshot {
  readonly at: number;
  readonly reading: FleetReading;
}

interface Save {
  readonly first: boolean;
  readonly key: string;
  readonly placement: Placement;
}

interface PlaceFailure {
  readonly key: string;
  readonly reason: string;
}

interface WindowContext {
  readonly agents: readonly Agent[];
  readonly asked: readonly string[];
  readonly at: number;
  readonly failures: readonly PlaceFailure[];
  readonly grips: readonly Grip[];
  readonly held: Layout;
  readonly openedKey: string;
  readonly saves: readonly Save[];
  readonly trouble: Trouble;
}

interface Press {
  readonly clientX: number;
  readonly clientY: number;
  readonly key: string;
}

interface Pointer {
  readonly clientX: number;
  readonly clientY: number;
}

interface Nudge {
  readonly key: string;
  readonly stepsX: number;
  readonly stepsY: number;
}

type WindowEvent =
  | (Nudge & { readonly type: "tile.nudge" })
  | (Pointer & { readonly type: "pointer.move" })
  | (Press & { readonly type: "tile.press" })
  | { readonly key: string; readonly type: "tile.click" | "workspace.open" }
  | {
      readonly type: "fleet.read" | "pointer.release" | "workspace.close";
    };

const NO_KEY = "";
const NO_TROUBLE: Trouble = { kind: "none" };
const NOTHING_TO_PLACE: Placement = { id: 0, location: "local", x: 0, y: 0 };

const INITIAL_CONTEXT: WindowContext = {
  agents: [],
  asked: [],
  at: 0,
  failures: [],
  grips: [],
  held: {},
  openedKey: NO_KEY,
  saves: [],
  trouble: NO_TROUBLE,
};

const tilesIn = (context: WindowContext): readonly Tile[] =>
  tilesOf({ agents: context.agents, now: context.at });

const layoutIn = (context: WindowContext): Layout =>
  shownLayout({ held: context.held, machines: tilesIn(context) });

const reasonOf = (error: unknown): string =>
  error instanceof Error
    ? error.message
    : "the window could not reach its own bridge";

const readingTaken = ({ at, reading }: FleetSnapshot): Partial<WindowContext> =>
  reading.kind === "agents"
    ? { agents: reading.agents, at, trouble: NO_TROUBLE }
    : { at, trouble: reading };

const readingFailed = (error: unknown): Partial<WindowContext> => ({
  trouble: { kind: "unreadable", reason: reasonOf(error) },
});

const savesFor = (
  context: WindowContext,
  keys: readonly string[],
  layout: Layout
): readonly Save[] =>
  Array.map(
    Array.filter(tilesIn(context), (tile) =>
      Array.some(keys, (key) => key === tile.key)
    ),
    ({ id, key, location, spot }) => ({
      first:
        spot === null && !Array.some(context.asked, (asked) => asked === key),
      key,
      placement: { id, location, ...positionOf({ key, layout }) },
    })
  );

const fleetSettled = (context: WindowContext): Partial<WindowContext> => {
  const machines = tilesIn(context);
  const waiting = stillHeld({ held: context.held, machines });
  const fresh = Array.filter(
    unplacedKeys(machines),
    (key) =>
      !Array.some(context.asked, (asked) => asked === key) &&
      !Array.some(context.grips, (grip) => grip.key === key)
  );
  const shown = shownLayout({ held: waiting, machines });
  const slots = Record.filter(shown, (_position, key) =>
    Array.some(fresh, (unplaced) => unplaced === key)
  );
  return {
    asked: [...context.asked, ...fresh],
    grips: Array.filter(context.grips, (grip) =>
      Array.some(machines, (machine) => machine.key === grip.key)
    ),
    held: { ...waiting, ...slots },
    saves: [...context.saves, ...savesFor(context, fresh, shown)],
  };
};

const isSaving = (context: WindowContext, key: string): boolean =>
  Array.some(context.saves, (save) => save.key === key);

const knowsTile = (context: WindowContext, key: string): boolean =>
  Array.some(tilesIn(context), (tile) => tile.key === key);

interface WithKey {
  readonly context: WindowContext;
  readonly key: string;
}

interface WithPress {
  readonly context: WindowContext;
  readonly press: Press;
}

interface WithPointer {
  readonly context: WindowContext;
  readonly pointer: Pointer;
}

interface WithNudge {
  readonly context: WindowContext;
  readonly nudge: Nudge;
}

interface WithOutcome {
  readonly context: WindowContext;
  readonly outcome: PlaceOutcome;
}

const canGrip = ({ context, key }: WithKey): boolean =>
  knowsTile(context, key) && !isSaving(context, key);

const canOpen = ({ context, key }: WithKey): boolean => knowsTile(context, key);

const agentLeft = (context: WindowContext): boolean =>
  !knowsTile(context, context.openedKey);

const gripped = ({ context, press }: WithPress): Partial<WindowContext> => ({
  failures: [],
  grips: [
    {
      key: press.key,
      origin: positionOf({ key: press.key, layout: layoutIn(context) }),
      startX: press.clientX,
      startY: press.clientY,
    },
  ],
});

const grippedTiles = (context: WindowContext): readonly Grip[] =>
  Array.filter(context.grips, (grip) => knowsTile(context, grip.key));

const pulledFar = ({ context, pointer }: WithPointer): boolean =>
  Array.some(context.grips, (grip) => pulledPast({ ...pointer, grip }));

const dragged = ({
  context,
  pointer,
}: WithPointer): Partial<WindowContext> => ({
  held: {
    ...context.held,
    ...Record.fromEntries(
      Array.map(grippedTiles(context), (grip) => [
        grip.key,
        draggedTo({ ...pointer, grip }),
      ])
    ),
  },
});

const released = (): Partial<WindowContext> => ({ grips: [] });

const movedKeys = (
  context: WindowContext,
  keys: readonly string[],
  layout: Layout
): readonly string[] =>
  Array.map(
    Array.filter(tilesIn(context), ({ key, spot }) => {
      const to = positionOf({ key, layout });
      return (
        Array.some(keys, (moved) => moved === key) &&
        !(spot !== null && spot.x === to.x && spot.y === to.y)
      );
    }),
    ({ key }) => key
  );

const settledMove = (
  context: WindowContext,
  keys: readonly string[],
  held: Layout
): Partial<WindowContext> => {
  const layout = shownLayout({ held, machines: tilesIn(context) });
  const moved = movedKeys(context, keys, layout);
  return {
    held: Record.filter(
      held,
      (_position, key) =>
        !Array.some(keys, (dropped) => dropped === key) ||
        Array.some(moved, (kept) => kept === key)
    ),
    saves: [...context.saves, ...savesFor(context, moved, layout)],
  };
};

const dropped = (context: WindowContext): Partial<WindowContext> => ({
  ...settledMove(
    context,
    Array.map(context.grips, ({ key }) => key),
    context.held
  ),
  grips: [],
});

const nudged = ({ context, nudge }: WithNudge): Partial<WindowContext> => {
  const { key, stepsX, stepsY } = nudge;
  const to = movedBy({
    deltaX: stepsToPixels(stepsX),
    deltaY: stepsToPixels(stepsY),
    from: positionOf({ key, layout: layoutIn(context) }),
  });
  return {
    ...settledMove(context, [key], Record.set(context.held, key, to)),
    failures: [],
  };
};

const hasSaves = (context: WindowContext): boolean =>
  Array.isReadonlyArrayNonEmpty(context.saves);

const nextPlacement = (context: WindowContext): Placement =>
  Option.match(Array.head(context.saves), {
    onNone: () => NOTHING_TO_PLACE,
    onSome: ({ placement }) => placement,
  });

const refusedSave = (
  context: WindowContext,
  sent: Save,
  outcome: Exclude<PlaceOutcome, { kind: "placed" }>
): Partial<WindowContext> => ({
  asked:
    outcome.kind === "daemon_down" && sent.first
      ? Array.filter(context.asked, (asked) => asked !== sent.key)
      : context.asked,
  failures: [{ key: sent.key, reason: outcome.reason }],
  held: Record.remove(context.held, sent.key),
  saves: Array.drop(context.saves, 1),
});

const saveSettled = ({
  context,
  outcome,
}: WithOutcome): Partial<WindowContext> =>
  Option.match(Array.head(context.saves), {
    onNone: () => ({}),
    onSome: (sent) =>
      outcome.kind === "placed"
        ? { failures: [], saves: Array.drop(context.saves, 1) }
        : refusedSave(context, sent, outcome),
  });

const wasPlaced = (outcome: PlaceOutcome): boolean => outcome.kind === "placed";

export {
  INITIAL_CONTEXT,
  NO_KEY,
  agentLeft,
  canGrip,
  canOpen,
  dragged,
  dropped,
  fleetSettled,
  gripped,
  hasSaves,
  layoutIn,
  nextPlacement,
  nudged,
  pulledFar,
  readingFailed,
  readingTaken,
  reasonOf,
  released,
  saveSettled,
  tilesIn,
  wasPlaced,
};
export type {
  FleetSnapshot,
  Nudge,
  PlaceFailure,
  Pointer,
  Press,
  Trouble,
  WindowContext,
  WindowEvent,
};
