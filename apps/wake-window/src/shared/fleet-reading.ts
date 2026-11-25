import type { AgentEntry } from "#/shared/agent.ts";
import type { DaemonAnswer } from "#/shared/daemon-answer.ts";
import { decodeAgents } from "#/shared/agent.ts";

type FleetReading =
  | { readonly kind: "agents"; readonly agents: readonly AgentEntry[] }
  | { readonly kind: "daemon_down"; readonly reason: string; readonly socket: string }
  | { readonly kind: "unreadable"; readonly reason: string };

const fleetReadingOf = (answer: DaemonAnswer): FleetReading => {
  if (answer.kind === "unreachable") {
    return { kind: "daemon_down", reason: answer.reason, socket: answer.socket };
  }
  if (answer.kind === "refused") {
    return { kind: "unreadable", reason: answer.reason };
  }
  const agents = decodeAgents(answer.result);
  if (!agents.ok) {
    return { kind: "unreadable", reason: agents.problem };
  }
  return { agents: agents.value, kind: "agents" };
};

export { fleetReadingOf };
export type { FleetReading };
