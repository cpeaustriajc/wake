import { Array, DateTime, Option, Record, String, pipe } from "effect";

import type { Agent, Position, Progress, Whereabouts } from "#/shared/agent.ts";

type LampTone = "action" | "danger" | "idle" | "success" | "warning";

interface Signal {
  readonly headline: string;
  readonly lamp: LampTone;
  readonly live: boolean;
  readonly status: string;
}

interface Tile extends Signal {
  readonly folder: string;
  readonly id: number;
  readonly key: string;
  readonly location: Whereabouts;
  readonly note: string;
  readonly place: string;
  readonly spot: Position | null;
  readonly when: string;
}

const HOURS_PER_DAY = 24;
const MILLISECONDS_PER_MINUTE = 60_000;
const MINUTES_PER_HOUR = 60;

const PHASE_LAMPS: Record.ReadonlyRecord<string, LampTone> = {
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

const gapLabel = (minutes: number): string => {
  if (minutes < 1) {
    return "just now";
  }
  if (minutes < MINUTES_PER_HOUR) {
    return `${minutes} min ago`;
  }
  const hours = Math.floor(minutes / MINUTES_PER_HOUR);
  return hours < HOURS_PER_DAY
    ? `${hours} h ago`
    : `${Math.floor(hours / HOURS_PER_DAY)} d ago`;
};

interface Since {
  readonly now: number;
  readonly updated: string;
}

interface Fleet {
  readonly agents: readonly Agent[];
  readonly now: number;
}

const whenOf = ({ now, updated }: Since): string =>
  Option.match(DateTime.make(updated), {
    onNone: () => "",
    onSome: (at) =>
      gapLabel(
        Math.floor((now - DateTime.toEpochMillis(at)) / MILLISECONDS_PER_MINUTE)
      ),
  });

const foundSignal = (headline: string, phase: string): Signal => {
  const word = pipe(phase, String.trim, String.toLowerCase);
  return {
    headline,
    lamp: Option.getOrElse(
      Record.get(PHASE_LAMPS, word),
      (): LampTone => "idle"
    ),
    live: word === "working",
    status: phase,
  };
};

const signalOf = (progress: Progress): Signal => {
  if (progress.state === "found") {
    return foundSignal(progress.headline, progress.phase);
  }
  if (progress.state === "invalid") {
    return {
      headline: "Invalid progress file",
      lamp: "danger",
      live: false,
      status: "Invalid file",
    };
  }
  if (progress.state === "unreachable") {
    return {
      headline: "Cannot read the remote",
      lamp: "warning",
      live: false,
      status: "Unreachable",
    };
  }
  return { ...QUIET_SIGNALS[progress.state], lamp: "idle", live: false };
};

const noteOf = ({ folder, progress }: Agent): string =>
  progress.state === "invalid" || progress.state === "unreachable"
    ? progress.reason
    : folder;

const keyOf = ({ id, location }: Pick<Agent, "id" | "location">): string =>
  location === "cloud" ? `c${id}` : `a${id}`;

const placeOf = ({ branch, location }: Agent): string =>
  location === "cloud" ? `Cloud, branch ${branch ?? "unknown"}` : "Local";

const tileOf = (agent: Agent, now: number): Tile => ({
  ...signalOf(agent.progress),
  folder: agent.folder,
  id: agent.id,
  key: keyOf(agent),
  location: agent.location,
  note: noteOf(agent),
  place: placeOf(agent),
  spot: agent.position ?? null,
  when:
    agent.progress.state === "found"
      ? whenOf({ now, updated: agent.progress.updated })
      : "",
});

const tilesOf = ({ agents, now }: Fleet): readonly Tile[] =>
  Array.map(agents, (agent) => tileOf(agent, now));

export { gapLabel, keyOf, tilesOf, whenOf };
export type { LampTone, Tile };
