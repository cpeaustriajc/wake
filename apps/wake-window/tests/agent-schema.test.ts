import { assert, describe, it } from "@effect/vitest";
import { Effect, Schema } from "effect";
import * as fc from "fast-check";

import { AgentList } from "#/shared/agent.ts";
import {
  decodeFleetReading,
  decodePlaceOutcome,
} from "#/shared/daemon-answer.ts";

const SERDE_OUTPUT =
  '{"agents":[{"id":1,"folder":"/work/good","location":"local","branch":null,"started_at":1795600000,"progress":{"state":"found","headline":"Adding the parser","phase":"build","updated":"2026-11-24T14:00:00Z"},"position":{"x":360,"y":40}},{"id":1,"folder":"/work/repo","location":"cloud","branch":"claude/task","started_at":1795600100,"progress":{"state":"branch_missing"},"position":null},{"id":2,"folder":"/work/broken","location":"local","branch":null,"started_at":1795600200,"progress":{"state":"invalid","reason":"the field `phase` is missing"}}]}';

const text = fc.string({ maxLength: 12 });
const whole = fc.integer({ max: 100_000, min: 0 });

const progress = fc.oneof(
  fc.record({
    headline: text,
    phase: text,
    state: fc.constant("found"),
    updated: text,
  }),
  fc.record({
    reason: text,
    state: fc.constantFrom("invalid", "unreachable"),
  }),
  fc.record({
    state: fc.constantFrom("branch_missing", "missing", "pending"),
  })
);

const wireAgent = fc.record(
  {
    branch: fc.option(text, { nil: null }),
    folder: text,
    id: whole,
    location: fc.constantFrom("cloud", "local"),
    position: fc.option(fc.record({ x: whole, y: whole }), { nil: null }),
    progress,
    started_at: whole,
  },
  {
    requiredKeys: [
      "branch",
      "folder",
      "id",
      "location",
      "progress",
      "started_at",
    ],
  }
);

const wireList = fc.record({ agents: fc.array(wireAgent, { maxLength: 6 }) });

const roundTrip = (wire: typeof AgentList.Encoded) =>
  Schema.decodeEffect(AgentList)(wire).pipe(
    Effect.flatMap((decoded) =>
      Schema.encodeEffect(AgentList)(decoded).pipe(
        Effect.flatMap(Schema.decodeUnknownEffect(AgentList)),
        Effect.map((again) => ({ again, decoded }))
      )
    )
  );

describe("the wire schemas", () => {
  it.effect("read the daemon's serde output, with and without a position", () =>
    Effect.gen(function* serde() {
      const wire = yield* Schema.decodeEffect(Schema.fromJsonString(AgentList))(
        SERDE_OUTPUT
      );
      assert.lengthOf(wire.agents, 3);
      assert.deepStrictEqual(wire.agents[0]?.position, { x: 360, y: 40 });
      assert.strictEqual(wire.agents[1]?.position, null);
      assert.isFalse("position" in (wire.agents[2] ?? {}));
      assert.strictEqual(wire.agents[1]?.branch, "claude/task");
    })
  );

  it.effect("decode, encode and decode any agent list to the same value", () =>
    Effect.promise(() =>
      fc.assert(
        fc.asyncProperty(wireList, (wire) =>
          Effect.runPromise(
            roundTrip(wire).pipe(
              Effect.tap(({ again, decoded }) =>
                Effect.sync(() => {
                  assert.deepStrictEqual(again, decoded);
                })
              ),
              Effect.asVoid
            )
          )
        )
      )
    )
  );

  it.effect(
    "turn anything that is not an answer into an unreadable reading, never a throw",
    () =>
      Effect.promise(() =>
        fc.assert(
          fc.asyncProperty(fc.jsonValue(), (wire) =>
            Effect.runPromise(
              Effect.all([
                decodeFleetReading(wire),
                decodePlaceOutcome(wire),
              ]).pipe(
                Effect.tap(([reading, outcome]) =>
                  Effect.sync(() => {
                    assert.deepStrictEqual(
                      [reading.kind, outcome.kind],
                      ["unreadable", "refused"]
                    );
                  })
                ),
                Effect.asVoid
              )
            )
          )
        )
      )
  );

  it.effect(
    "turn an answer whose agents do not fit into an unreadable reading",
    () =>
      Effect.gen(function* misfit() {
        const reading = yield* decodeFleetReading({
          kind: "answered",
          result: { agents: [{ id: "one" }] },
        });
        const down = yield* decodeFleetReading({
          kind: "unreachable",
          reason: "connect ENOENT",
          socket: "/tmp/wake.sock",
        });
        assert.strictEqual(reading.kind, "unreadable");
        assert.deepStrictEqual(down, {
          kind: "daemon_down",
          reason: "connect ENOENT",
          socket: "/tmp/wake.sock",
        });
      })
  );
});
