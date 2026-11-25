interface WakeBridge {
  readonly openTerminal: (opening: unknown) => Promise<unknown>;
  readonly placeOnFloor: (placement: unknown) => Promise<unknown>;
  readonly readFleet: () => Promise<unknown>;
  readonly readTerminal: (reading: unknown) => Promise<unknown>;
  readonly resizeTerminal: (size: unknown) => Promise<unknown>;
  readonly writeTerminal: (input: unknown) => Promise<unknown>;
}

const BRIDGE_NAME = "wake";
const LIST_AGENTS_CHANNEL = "wake:agent.list";
const PLACE_ON_FLOOR_CHANNEL = "wake:floor.place";
const OPEN_TERMINAL_CHANNEL = "wake:terminal.open";
const READ_TERMINAL_CHANNEL = "wake:terminal.read";
const RESIZE_TERMINAL_CHANNEL = "wake:terminal.resize";
const WRITE_TERMINAL_CHANNEL = "wake:terminal.write";

export {
  BRIDGE_NAME,
  LIST_AGENTS_CHANNEL,
  OPEN_TERMINAL_CHANNEL,
  PLACE_ON_FLOOR_CHANNEL,
  READ_TERMINAL_CHANNEL,
  RESIZE_TERMINAL_CHANNEL,
  WRITE_TERMINAL_CHANNEL,
};
export type { WakeBridge };
