import { Schema } from "effect";

const Location = Schema.Literals(["cloud", "local"]);

const FloorPosition = Schema.Struct({ x: Schema.Int, y: Schema.Int });

const AgentProgress = Schema.Union([
  Schema.Struct({
    headline: Schema.String,
    phase: Schema.String,
    state: Schema.Literal("found"),
    updated: Schema.String,
  }),
  Schema.Struct({
    reason: Schema.String,
    state: Schema.Literals(["invalid", "unreachable"]),
  }),
  Schema.Struct({
    state: Schema.Literals(["branch_missing", "missing", "pending"]),
  }),
]);

const AgentEntry = Schema.Struct({
  branch: Schema.NullOr(Schema.String),
  folder: Schema.String,
  id: Schema.Int,
  location: Location,
  position: FloorPosition.pipe(Schema.NullOr, Schema.optionalKey),
  progress: AgentProgress,
  started_at: Schema.Int,
});

const AgentList = Schema.Struct({ agents: Schema.Array(AgentEntry) });

const FloorPlacement = Schema.Struct({
  id: Schema.Int,
  location: Location,
  x: Schema.Int,
  y: Schema.Int,
});

type Agent = typeof AgentEntry.Type;
type Placement = typeof FloorPlacement.Type;
type Position = typeof FloorPosition.Type;
type Progress = typeof AgentProgress.Type;
type Whereabouts = typeof Location.Type;

export {
  AgentEntry,
  AgentList,
  AgentProgress,
  FloorPlacement,
  FloorPosition,
  Location,
};
export type { Agent, Placement, Position, Progress, Whereabouts };
