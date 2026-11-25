import type { FleetReading } from "#/shared/fleet-reading.ts";

interface WakeBridge {
  readonly readFleet: () => Promise<FleetReading>;
}

const LIST_AGENTS_CHANNEL = "wake:agent.list";
const BRIDGE_NAME = "wake";

export { BRIDGE_NAME, LIST_AGENTS_CHANNEL };
export type { WakeBridge };
