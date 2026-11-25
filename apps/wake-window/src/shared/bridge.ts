import type { FloorPlacement, PlaceOutcome } from "#/shared/floor-placement.ts";
import type { FleetReading } from "#/shared/fleet-reading.ts";

interface WakeBridge {
  readonly placeOnFloor: (placement: FloorPlacement) => Promise<PlaceOutcome>;
  readonly readFleet: () => Promise<FleetReading>;
}

const BRIDGE_NAME = "wake";
const LIST_AGENTS_CHANNEL = "wake:agent.list";
const PLACE_ON_FLOOR_CHANNEL = "wake:floor.place";

export { BRIDGE_NAME, LIST_AGENTS_CHANNEL, PLACE_ON_FLOOR_CHANNEL };
export type { WakeBridge };
