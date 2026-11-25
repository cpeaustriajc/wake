import type { AgentEntry, AgentProgress } from "#/shared/agent.ts";

type LampTone = "action" | "danger" | "idle" | "success" | "warning";

interface Tile {
  readonly cloud: boolean;
  readonly folder: string;
  readonly headline: string;
  readonly key: string;
  readonly lamp: LampTone;
  readonly live: boolean;
  readonly note: string;
  readonly place: string;
  readonly status: string;
  readonly when: string;
}

interface Signal {
  readonly headline: string;
  readonly lamp: LampTone;
  readonly live: boolean;
  readonly status: string;
}

const FIRST_MINUTE = 1;
const HOURS_PER_DAY = 24;
const MILLISECONDS_PER_MINUTE = 60_000;
const MINUTES_PER_HOUR = 60;

const PHASE_LAMPS: Readonly<Record<string, LampTone>> = {
  done: "action",
  planning: "success",
  review: "warning",
  working: "success",
};

const QUIET_SIGNALS = {
  branch_missing: { headline: "Branch not pushed yet", status: "Not pushed" },
  missing: { headline: "No progress file", status: "No file" },
  pending: { headline: "Not read yet", status: "Reading" },
} as const;

const longAgo = (minutes: number): string => {
  const hours = Math.floor(minutes / MINUTES_PER_HOUR);
  if (hours < HOURS_PER_DAY) {
    return `${hours} h ago`;
  }
  return `${Math.floor(hours / HOURS_PER_DAY)} d ago`;
};

const whenOf = (updated: string, now: number): string => {
  const minutes = Math.floor((now - Date.parse(updated)) / MILLISECONDS_PER_MINUTE);
  if (Number.isNaN(minutes)) {
    return "";
  }
  if (minutes < FIRST_MINUTE) {
    return "just now";
  }
  if (minutes < MINUTES_PER_HOUR) {
    return `${minutes} min ago`;
  }
  return longAgo(minutes);
};

const foundSignal = (headline: string, phase: string): Signal => {
  const word = phase.trim().toLowerCase();
  return { headline, lamp: PHASE_LAMPS[word] ?? "idle", live: word === "working", status: phase };
};

const signalOf = (progress: AgentProgress): Signal => {
  if (progress.state === "found") {
    return foundSignal(progress.headline, progress.phase);
  }
  if (progress.state === "invalid") {
    return { headline: "Invalid progress file", lamp: "danger", live: false, status: "Invalid file" };
  }
  if (progress.state === "unreachable") {
    return { headline: "Cannot read the remote", lamp: "warning", live: false, status: "Unreachable" };
  }
  if (progress.state === "unrecognized") {
    return { headline: "Unknown progress state", lamp: "idle", live: false, status: progress.name };
  }
  return { ...QUIET_SIGNALS[progress.state], lamp: "idle", live: false };
};

const noteOf = (agent: AgentEntry): string => {
  if (agent.progress.state === "invalid" || agent.progress.state === "unreachable") {
    return agent.progress.reason;
  }
  return agent.folder;
};

const updatedOf = (progress: AgentProgress, now: number): string => {
  if (progress.state === "found") {
    return whenOf(progress.updated, now);
  }
  return "";
};

const tileOf = (agent: AgentEntry, now: number): Tile => {
  const shared = {
    ...signalOf(agent.progress),
    folder: agent.folder,
    note: noteOf(agent),
    when: updatedOf(agent.progress, now),
  };
  if (agent.place.location === "cloud") {
    return { ...shared, cloud: true, key: `c${agent.id}`, place: `Cloud, branch ${agent.place.branch}` };
  }
  return { ...shared, cloud: false, key: `a${agent.id}`, place: "Local" };
};

const tilesOf = (agents: readonly AgentEntry[], now: number): readonly Tile[] =>
  agents.map((agent) => tileOf(agent, now));

export { tileOf, tilesOf, whenOf };
export type { LampTone, Tile };
