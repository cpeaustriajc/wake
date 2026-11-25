import type { AgentEntry } from "#/shared/agent.ts";
import type { FleetReading } from "#/shared/fleet-reading.ts";

type Trouble =
  | { readonly kind: "daemon_down"; readonly reason: string; readonly socket: string }
  | { readonly kind: "none" }
  | { readonly kind: "unreadable"; readonly reason: string };

interface FleetSnapshot {
  readonly at: number;
  readonly reading: FleetReading;
}

interface FleetContext {
  readonly agents: readonly AgentEntry[];
  readonly at: number;
  readonly trouble: Trouble;
}

const NEVER_READ = 0;
const NO_TROUBLE: Trouble = { kind: "none" };
const INITIAL_FLEET: FleetContext = { agents: [], at: NEVER_READ, trouble: NO_TROUBLE };

const reasonOf = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  return "the window could not reach its own bridge";
};

const readingTaken = (snapshot: FleetSnapshot): Partial<FleetContext> => {
  const { at, reading } = snapshot;
  if (reading.kind === "agents") {
    return { agents: reading.agents, at, trouble: NO_TROUBLE };
  }
  return { at, trouble: reading };
};

const readingFailed = (error: unknown): Pick<FleetContext, "trouble"> => ({
  trouble: { kind: "unreadable", reason: reasonOf(error) },
});

export { INITIAL_FLEET, readingFailed, readingTaken, reasonOf };
export type { FleetContext, FleetSnapshot, Trouble };
