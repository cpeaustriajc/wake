interface WakeBridge {
  readonly placeOnFloor: (placement: unknown) => Promise<unknown>;
  readonly readFleet: () => Promise<unknown>;
}

const BRIDGE_NAME = "wake";
const LIST_AGENTS_CHANNEL = "wake:agent.list";
const PLACE_ON_FLOOR_CHANNEL = "wake:floor.place";

export { BRIDGE_NAME, LIST_AGENTS_CHANNEL, PLACE_ON_FLOOR_CHANNEL };
export type { WakeBridge };
