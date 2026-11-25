import { failure, success } from "#/shared/outcome.ts";
import { isRecord, textOf } from "#/shared/json.ts";
import type { JsonRecord } from "#/shared/json.ts";
import type { Outcome } from "#/shared/outcome.ts";

type AgentProgress =
  | {
      readonly state: "found";
      readonly headline: string;
      readonly phase: string;
      readonly updated: string;
    }
  | { readonly state: "invalid" | "unreachable"; readonly reason: string }
  | { readonly state: "branch_missing" | "missing" | "pending" }
  | { readonly state: "unrecognized"; readonly name: string };

type AgentPlace =
  | { readonly location: "cloud"; readonly branch: string }
  | { readonly location: "local" };

interface AgentEntry {
  readonly id: number;
  readonly folder: string;
  readonly place: AgentPlace;
  readonly startedAt: number;
  readonly progress: AgentProgress;
}

const NO_REASON = "no reason given";

const decodeFound = (progress: JsonRecord): Outcome<AgentProgress> => {
  const { headline, phase, updated } = progress;
  if (typeof headline !== "string" || typeof phase !== "string" || typeof updated !== "string") {
    return failure("a found progress file lacks headline, phase or updated");
  }
  return success({ headline, phase, state: "found", updated });
};

const decodeProgress = (progress: unknown): Outcome<AgentProgress> => {
  if (!isRecord(progress) || typeof progress["state"] !== "string") {
    return failure("an agent's progress has no state");
  }
  const { state } = progress;
  switch (state) {
    case "found": {
      return decodeFound(progress);
    }
    case "invalid":
    case "unreachable": {
      return success({ reason: textOf(progress["reason"], NO_REASON), state });
    }
    case "branch_missing":
    case "missing":
    case "pending": {
      return success({ state });
    }
    default: {
      return success({ name: state, state: "unrecognized" });
    }
  }
};

const decodePlace = (entry: JsonRecord): Outcome<AgentPlace> => {
  const { branch, location } = entry;
  if (location === "local") {
    return success({ location });
  }
  if (location === "cloud" && typeof branch === "string") {
    return success({ branch, location });
  }
  return failure("an agent is neither local nor a cloud task with a branch");
};

const decodeSituation = (entry: JsonRecord): Outcome<Pick<AgentEntry, "place" | "progress">> => {
  const place = decodePlace(entry);
  if (!place.ok) {
    return place;
  }
  const progress = decodeProgress(entry["progress"]);
  if (!progress.ok) {
    return progress;
  }
  return success({ place: place.value, progress: progress.value });
};

const decodeAgent = (entry: unknown): Outcome<AgentEntry> => {
  if (!isRecord(entry)) {
    return failure("an agent is not an object");
  }
  const { folder, id, started_at: startedAt } = entry;
  if (typeof folder !== "string" || typeof id !== "number" || typeof startedAt !== "number") {
    return failure("an agent lacks folder, id or started_at");
  }
  const situation = decodeSituation(entry);
  if (!situation.ok) {
    return situation;
  }
  return success({ folder, id, startedAt, ...situation.value });
};

const decodeAgents = (result: unknown): Outcome<readonly AgentEntry[]> => {
  if (!isRecord(result) || !Array.isArray(result["agents"])) {
    return failure("the answer has no list of agents");
  }
  const agents: AgentEntry[] = [];
  for (const entry of result["agents"]) {
    const agent = decodeAgent(entry);
    if (!agent.ok) {
      return agent;
    }
    agents.push(agent.value);
  }
  return success(agents);
};

export { decodeAgents };
export type { AgentEntry, AgentPlace, AgentProgress };
