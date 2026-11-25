import type { FloorPlacement, PlaceOutcome } from "#/shared/floor-placement.ts";
import type { Grip, Layout, Position } from "#/renderer/floor/floor-model.ts";
import {
  draggedTo,
  movedBy,
  positionOf,
  pulledPast,
  shownLayout,
  stepsToPixels,
  stillHeld,
  unplacedKeys,
  withPosition,
  withoutPosition,
} from "#/renderer/floor/floor-model.ts";
import type { AgentEntry } from "#/shared/agent.ts";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import { tilesOf } from "#/renderer/floor/tile-model.ts";

interface Save {
  readonly key: string;
  readonly placement: FloorPlacement;
}

interface PlaceFailure {
  readonly key: string;
  readonly reason: string;
}

interface PlacementContext {
  readonly agents: readonly AgentEntry[];
  readonly asked: readonly string[];
  readonly at: number;
  readonly failures: readonly PlaceFailure[];
  readonly grips: readonly Grip[];
  readonly held: Layout;
  readonly saves: readonly Save[];
}

interface Press {
  readonly clientX: number;
  readonly clientY: number;
  readonly key: string;
}

interface Pull {
  readonly clientX: number;
  readonly clientY: number;
}

interface Nudge {
  readonly key: string;
  readonly stepsLeft: number;
  readonly stepsTop: number;
}

const NO_AGENT = 0;
const NOTHING_TO_PLACE: FloorPlacement = {
  id: NO_AGENT,
  left: NO_AGENT,
  location: "local",
  top: NO_AGENT,
};

const INITIAL_PLACEMENT: Pick<PlacementContext, "asked" | "failures" | "grips" | "held" | "saves"> =
  { asked: [], failures: [], grips: [], held: {}, saves: [] };

const tilesIn = (context: Pick<PlacementContext, "agents" | "at">): readonly Tile[] =>
  tilesOf(context.agents, context.at);

const layoutIn = (context: Pick<PlacementContext, "agents" | "at" | "held">): Layout =>
  shownLayout(tilesIn(context), context.held);

const savesFor = (
  context: Pick<PlacementContext, "agents" | "at">,
  keys: readonly string[],
  layout: Layout,
): readonly Save[] =>
  tilesIn(context)
    .filter((tile) => keys.includes(tile.key))
    .map((tile) => ({
      key: tile.key,
      placement: { id: tile.id, location: tile.location, ...positionOf(layout, tile.key) },
    }));

const fleetSettled = (
  context: PlacementContext,
): Pick<PlacementContext, "asked" | "held" | "saves"> => {
  const tiles = tilesIn(context);
  const waiting = stillHeld(context.held, tiles);
  const fresh = unplacedKeys(tiles).filter((key) => !context.asked.includes(key));
  const shown = shownLayout(tiles, waiting);
  const slots = fresh.map((key): [string, Position] => [key, positionOf(shown, key)]);
  return {
    asked: [...context.asked, ...fresh],
    held: { ...waiting, ...Object.fromEntries(slots) },
    saves: [...context.saves, ...savesFor(context, fresh, shown)],
  };
};

const isSaving = (context: Pick<PlacementContext, "saves">, key: string): boolean =>
  context.saves.some((save) => save.key === key);

const canGrip = (context: PlacementContext, key: string): boolean =>
  !isSaving(context, key) && tilesIn(context).some((tile) => tile.key === key);

const gripped = (
  context: PlacementContext,
  press: Press,
): Pick<PlacementContext, "failures" | "grips"> => {
  const { clientX: startX, clientY: startY, key } = press;
  return {
    failures: [],
    grips: [{ key, origin: positionOf(layoutIn(context), key), startX, startY }],
  };
};

const pulledFar = (context: Pick<PlacementContext, "grips">, pull: Pull): boolean =>
  context.grips.some((grip) => pulledPast(grip, pull.clientX, pull.clientY));

const dragged = (context: PlacementContext, pull: Pull): Pick<PlacementContext, "held"> => {
  const pulled = context.grips.map((grip): [string, Position] => [
    grip.key,
    draggedTo(grip, pull.clientX, pull.clientY),
  ]);
  return { held: { ...context.held, ...Object.fromEntries(pulled) } };
};

const released = (): Pick<PlacementContext, "grips"> => ({ grips: [] });

const dropped = (context: PlacementContext): Pick<PlacementContext, "grips" | "saves"> => {
  const keys = context.grips.map((grip) => grip.key);
  return {
    grips: [],
    saves: [...context.saves, ...savesFor(context, keys, layoutIn(context))],
  };
};

const nudged = (
  context: PlacementContext,
  nudge: Nudge,
): Pick<PlacementContext, "failures" | "held" | "saves"> => {
  const { key, stepsLeft, stepsTop } = nudge;
  const from = positionOf(layoutIn(context), key);
  const to = movedBy(from, stepsToPixels(stepsLeft), stepsToPixels(stepsTop));
  return {
    failures: [],
    held: withPosition(context.held, key, to),
    saves: [...context.saves, ...savesFor(context, [key], { [key]: to })],
  };
};

const hasSaves = (context: Pick<PlacementContext, "saves">): boolean =>
  context.saves.some((save) => save.key !== "");

const nextPlacement = (context: Pick<PlacementContext, "saves">): FloorPlacement => {
  const [next] = context.saves;
  if (!next) {
    return NOTHING_TO_PLACE;
  }
  return next.placement;
};

const askedAfter = (asked: readonly string[], key: string, outcome: PlaceOutcome): readonly string[] => {
  if (outcome.kind === "daemon_down") {
    return asked.filter((earlier) => earlier !== key);
  }
  return asked;
};

const saveSettled = (
  context: PlacementContext,
  outcome: PlaceOutcome,
): Partial<Pick<PlacementContext, "asked" | "failures" | "held" | "saves">> => {
  const [sent, ...saves] = context.saves;
  if (!sent || outcome.kind === "placed") {
    return { failures: [], saves };
  }
  return {
    asked: askedAfter(context.asked, sent.key, outcome),
    failures: [{ key: sent.key, reason: outcome.reason }],
    held: withoutPosition(context.held, sent.key),
    saves,
  };
};

export {
  INITIAL_PLACEMENT,
  canGrip,
  dragged,
  dropped,
  fleetSettled,
  gripped,
  hasSaves,
  layoutIn,
  nextPlacement,
  nudged,
  pulledFar,
  released,
  saveSettled,
  tilesIn,
};
export type { Nudge, PlaceFailure, PlacementContext, Press, Pull, Save };
