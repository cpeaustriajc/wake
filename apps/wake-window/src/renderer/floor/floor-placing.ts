import {
  EMPTY_LAYOUT,
  positionOf,
  shownLayout,
  stillHeld,
  unplacedKeys,
  withPosition,
  withoutPosition,
} from "#/renderer/floor/floor-model.ts";
import type { Layout, Position } from "#/renderer/floor/floor-model.ts";
import { createEffect, createMemo, createSignal } from "solid-js";
import type { Accessor } from "solid-js";
import type { PlaceOutcome } from "#/shared/floor-placement.ts";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import type { WakeBridge } from "#/shared/bridge.ts";

interface PlaceFailure {
  readonly key: string;
  readonly reason: string;
}

interface Placing {
  readonly drop: (key: string, position: Position) => void;
  readonly failures: Accessor<readonly PlaceFailure[]>;
  readonly layout: Accessor<Layout>;
  readonly move: (key: string, position: Position) => void;
}

const NO_FAILURES: readonly PlaceFailure[] = [];

const reasonOf = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  return "the window could not reach its own bridge";
};

const placed = async (
  bridge: WakeBridge,
  tile: Tile,
  position: Position,
): Promise<PlaceOutcome> => {
  try {
    return await bridge.placeOnFloor({ id: tile.id, location: tile.location, ...position });
  } catch (error) {
    return { kind: "refused", reason: reasonOf(error) };
  }
};

const createPlacing = (bridge: WakeBridge, tiles: Accessor<readonly Tile[]>): Placing => {
  const asked = new Set<string>();
  const [held, setHeld] = createSignal(EMPTY_LAYOUT);
  const [failures, setFailures] = createSignal(NO_FAILURES);
  const send = async (tile: Tile, position: Position): Promise<void> => {
    const outcome = await placed(bridge, tile, position);
    if (outcome.kind === "placed") {
      setFailures(NO_FAILURES);
      return;
    }
    if (outcome.kind === "daemon_down") {
      asked.delete(tile.key);
    }
    setHeld((before) => withoutPosition(before, tile.key));
    setFailures([{ key: tile.key, reason: outcome.reason }]);
  };
  const sendAll = (current: readonly Tile[], keys: readonly string[], shown: Layout): void => {
    for (const tile of current.filter((candidate) => keys.includes(candidate.key))) {
      asked.add(tile.key);
      void send(tile, positionOf(shown, tile.key));
    }
  };
  createEffect(tiles, (current) => {
    const waiting = stillHeld(held(), current);
    const fresh = unplacedKeys(current).filter((key) => !asked.has(key));
    const shown = shownLayout(current, waiting);
    setHeld(
      fresh.reduce((layout, key) => withPosition(layout, key, positionOf(shown, key)), waiting),
    );
    sendAll(current, fresh, shown);
  });
  return {
    drop: (key, position): void => {
      setHeld((before) => withPosition(before, key, position));
      sendAll(tiles(), [key], { [key]: position });
    },
    failures,
    layout: createMemo(() => shownLayout(tiles(), held())),
    move: (key, position): void => {
      setHeld((before) => withPosition(before, key, position));
    },
  };
};

export { createPlacing };
export type { PlaceFailure, Placing };
