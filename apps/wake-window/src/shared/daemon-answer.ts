import { Effect, Schema } from "effect";

import { AgentList } from "#/shared/agent.ts";
import type { Agent } from "#/shared/agent.ts";

const DaemonAnswer = Schema.Union([
  Schema.Struct({ kind: Schema.Literal("answered"), result: Schema.Unknown }),
  Schema.Struct({ kind: Schema.Literal("refused"), reason: Schema.String }),
  Schema.Struct({
    kind: Schema.Literal("unreachable"),
    reason: Schema.String,
    socket: Schema.String,
  }),
]);

type Answer = typeof DaemonAnswer.Type;

type FleetReading =
  | { readonly agents: readonly Agent[]; readonly kind: "agents" }
  | {
      readonly kind: "daemon_down";
      readonly reason: string;
      readonly socket: string;
    }
  | { readonly kind: "unreadable"; readonly reason: string };

type PlaceOutcome =
  | { readonly kind: "daemon_down" | "refused"; readonly reason: string }
  | { readonly kind: "placed" };

const agentsIn = (result: unknown): Effect.Effect<FleetReading> =>
  Schema.decodeUnknownEffect(AgentList)(result).pipe(
    Effect.match({
      onFailure: (error): FleetReading => ({
        kind: "unreadable",
        reason: error.message,
      }),
      onSuccess: ({ agents }): FleetReading => ({ agents, kind: "agents" }),
    })
  );

const fleetReadingOf = (answer: Answer): Effect.Effect<FleetReading> => {
  if (answer.kind === "answered") {
    return agentsIn(answer.result);
  }
  if (answer.kind === "unreachable") {
    return Effect.succeed({
      kind: "daemon_down",
      reason: answer.reason,
      socket: answer.socket,
    });
  }
  return Effect.succeed({ kind: "unreadable", reason: answer.reason });
};

const placeOutcomeOf = (answer: Answer): PlaceOutcome => {
  if (answer.kind === "answered") {
    return { kind: "placed" };
  }
  if (answer.kind === "unreachable") {
    return { kind: "daemon_down", reason: answer.reason };
  }
  return { kind: "refused", reason: answer.reason };
};

const decodeFleetReading = (wire: unknown): Effect.Effect<FleetReading> =>
  Schema.decodeUnknownEffect(DaemonAnswer)(wire).pipe(
    Effect.flatMap(fleetReadingOf),
    Effect.orElseSucceed((): FleetReading => ({
      kind: "unreadable",
      reason: "the window's own bridge sent something that is not an answer",
    }))
  );

const decodePlaceOutcome = (wire: unknown): Effect.Effect<PlaceOutcome> =>
  Schema.decodeUnknownEffect(DaemonAnswer)(wire).pipe(
    Effect.map(placeOutcomeOf),
    Effect.orElseSucceed((): PlaceOutcome => ({
      kind: "refused",
      reason: "the window's own bridge sent something that is not an answer",
    }))
  );

export { DaemonAnswer, decodeFleetReading, decodePlaceOutcome };
export type { Answer, FleetReading, PlaceOutcome };
