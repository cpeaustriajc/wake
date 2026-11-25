import { createMemo, createSignal, onSettled } from "solid-js";
import type { Accessor } from "solid-js";
import type { FleetReading } from "#/shared/fleet-reading.ts";
import type { Placing } from "#/renderer/floor/floor-placing.ts";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import type { WakeBridge } from "#/shared/bridge.ts";
import { createPlacing } from "#/renderer/floor/floor-placing.ts";
import { tilesOf } from "#/renderer/floor/tile-model.ts";

interface Snapshot {
  readonly at: number;
  readonly reading: FleetReading;
}

interface FloorState {
  readonly placing: Placing;
  readonly reading: Accessor<FleetReading>;
  readonly tiles: Accessor<readonly Tile[]>;
}

const POLL_MILLISECONDS = 2000;

const createSnapshot = (bridge: WakeBridge): Accessor<Snapshot> => {
  const [now, setNow] = createSignal(Date.now());
  onSettled(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, POLL_MILLISECONDS);
    return (): void => {
      clearInterval(timer);
    };
  });
  return createMemo(async () => {
    const at = now();
    const reading = await bridge.readFleet();
    return { at, reading };
  });
};

const tilesIn = (snapshot: Snapshot): readonly Tile[] => {
  if (snapshot.reading.kind !== "agents") {
    return [];
  }
  return tilesOf(snapshot.reading.agents, snapshot.at);
};

const createFloorState = (bridge: WakeBridge): FloorState => {
  const snapshot = createSnapshot(bridge);
  const tiles = createMemo(() => tilesIn(snapshot()));
  return {
    placing: createPlacing(bridge, tiles),
    reading: (): FleetReading => snapshot().reading,
    tiles,
  };
};

export { createFloorState };
