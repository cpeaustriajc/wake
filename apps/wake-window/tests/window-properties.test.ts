import { assert, describe, it } from "@effect/vitest";
import { Array, Effect, Option, Record } from "effect";
import * as fc from "fast-check";
import { SimulatedClock, createActor, fromPromise, waitFor } from "xstate";

import { tilesIn } from "#/renderer/window/window-context-model.ts";
import type {
  FleetSnapshot,
  WindowEvent,
} from "#/renderer/window/window-context-model.ts";
import { windowMachine } from "#/renderer/window/window-machine.ts";
import type { WindowSnapshot } from "#/renderer/window/window-machine.ts";
import type { Agent, Placement, Position } from "#/shared/agent.ts";
import type { PlaceOutcome } from "#/shared/daemon-answer.ts";

type Step =
  | { readonly event: WindowEvent; readonly kind: "event" }
  | { readonly kind: "leave" | "poll" | "return" };

interface World {
  readonly agents: Map<string, Agent>;
  readonly sent: Placement[];
}

const POLL = 2000;
const KEYS = ["a1", "a2", "a3", "a9"] as const;

const seed = (id: number, position: Position | null): Agent => ({
  branch: null,
  folder: `/work/${id}`,
  id,
  location: "local",
  position,
  progress: { state: "missing" },
  started_at: id,
});

const key = fc.constantFrom(...KEYS);
const point = fc.integer({ max: 900, min: -50 });

const step: fc.Arbitrary<Step> = fc.oneof(
  fc.record({
    event: fc.record({
      clientX: point,
      clientY: point,
      key,
      type: fc.constant("tile.press" as const),
    }),
    kind: fc.constant("event" as const),
  }),
  fc.record({
    event: fc.record({
      clientX: point,
      clientY: point,
      type: fc.constant("pointer.move" as const),
    }),
    kind: fc.constant("event" as const),
  }),
  fc.constant<Step>({ event: { type: "pointer.release" }, kind: "event" }),
  fc.record({
    event: fc.record({ key, type: fc.constant("tile.click" as const) }),
    kind: fc.constant("event" as const),
  }),
  fc.record({
    event: fc.record({
      key,
      stepsX: fc.integer({ max: 1, min: -1 }),
      stepsY: fc.integer({ max: 1, min: -1 }),
      type: fc.constant("tile.nudge" as const),
    }),
    kind: fc.constant("event" as const),
  }),
  fc.constant<Step>({ event: { type: "workspace.close" }, kind: "event" }),
  fc.constantFrom<Step>({ kind: "poll" }, { kind: "leave" }, { kind: "return" })
);

const settled = (
  actor: ReturnType<typeof createActor<typeof windowMachine>>
): Promise<WindowSnapshot> =>
  waitFor(
    actor,
    (snapshot) =>
      snapshot.matches({ fleet: { cycle: "waiting" } }) &&
      !snapshot.matches({ placement: { save: "saving" } })
  );

const holds = (snapshot: WindowSnapshot): void => {
  const { context } = snapshot;
  const tiles = tilesIn(context);
  const inHand = Array.map(context.grips, (grip) => grip.key);
  assert.deepStrictEqual(
    Array.filter(context.saves, (save) =>
      Array.some(inHand, (held) => held === save.key)
    ),
    [],
    "A tile is never both being dragged and being saved"
  );
  assert.deepStrictEqual(
    Array.filter(Record.toEntries(context.held), ([held, position]) =>
      Option.match(
        Array.findFirst(tiles, (tile) => tile.key === held),
        {
          onNone: () => true,
          onSome: ({ spot }) =>
            !Array.some(inHand, (gripped) => gripped === held) &&
            spot?.x === position.x &&
            spot.y === position.y,
        }
      )
    ),
    [],
    "The pending set contains only tiles with an unconfirmed save"
  );
  assert.isTrue(
    !snapshot.matches({ navigation: "workspace" }) ||
      Array.some(tiles, (tile) => tile.key === context.openedKey),
    "The workspace is never open for an agent that left the fleet"
  );
};

const play = (steps: readonly Step[], refusals: readonly boolean[]) => {
  const world: World = {
    agents: new Map([
      ["a1", seed(1, { x: 40, y: 40 })],
      ["a2", seed(2, { x: 360, y: 40 })],
      ["a3", seed(3, null)],
    ]),
    sent: [],
  };
  const away = new Map<string, Agent>();
  const clock = new SimulatedClock();
  const outcome = (placement: Placement): PlaceOutcome => {
    const refused = refusals[world.sent.length % Math.max(1, refusals.length)];
    world.sent.push(placement);
    const target = world.agents.get(`a${placement.id}`);
    if (refused === true || target === undefined) {
      return { kind: "refused", reason: "the daemon said no" };
    }
    world.agents.set(`a${placement.id}`, {
      ...target,
      position: { x: placement.x, y: placement.y },
    });
    return { kind: "placed" };
  };
  const actor = createActor(
    windowMachine.provide({
      actors: {
        placeOnFloor: fromPromise(({ input }) =>
          Promise.resolve(outcome(input))
        ),
        readFleet: fromPromise(() =>
          Promise.resolve<FleetSnapshot>({
            at: clock.now(),
            reading: { agents: [...world.agents.values()], kind: "agents" },
          })
        ),
      },
    }),
    { clock }
  );
  const apply = (next: Step): Promise<WindowSnapshot> => {
    if (next.kind === "event") {
      actor.send(next.event);
    } else if (next.kind === "poll") {
      clock.increment(POLL);
    } else if (next.kind === "leave") {
      const gone = world.agents.get("a2");
      if (gone !== undefined) {
        away.set("a2", gone);
        world.agents.delete("a2");
      }
    } else {
      const back = away.get("a2");
      if (back !== undefined) {
        world.agents.set("a2", back);
        away.delete("a2");
      }
    }
    return settled(actor);
  };
  actor.start();
  return Effect.promise(() => settled(actor)).pipe(
    Effect.flatMap(() =>
      Effect.forEach(steps, (next) =>
        Effect.promise(() => apply(next)).pipe(Effect.map(holds))
      )
    ),
    Effect.map(() => {
      actor.stop();
      return world;
    })
  );
};

const stepsOf = fc.array(step, { maxLength: 30 });
const refusalsOf = fc.array(fc.boolean(), { maxLength: 4 });

describe("the window machine, under any sequence of events", () => {
  it.effect(
    "keeps its promises about dragging, pending positions and the workspace",
    () =>
      Effect.promise(() =>
        fc.assert(
          fc.asyncProperty(stepsOf, refusalsOf, (steps, refusals) =>
            play(steps, refusals).pipe(Effect.asVoid, Effect.runPromise)
          )
        )
      )
  );

  it.effect(
    "An unplaced agent is sent to the daemon at most once, not counting sends the daemon never received",
    () =>
      Effect.promise(() =>
        fc.assert(
          fc.asyncProperty(
            fc.array(fc.constantFrom<Step>({ kind: "poll" }), { maxLength: 8 }),
            fc.boolean(),
            (polls, refuse) =>
              Effect.runPromise(
                play(polls, [refuse]).pipe(
                  Effect.tap(({ sent }) =>
                    Effect.sync(() => {
                      assert.lengthOf(
                        Array.filter(sent, (placement) => placement.id === 3),
                        1
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

  it.effect("A press that never moves 5 px never places anything", () =>
    Effect.promise(() =>
      fc.assert(
        fc.asyncProperty(
          fc.array(
            fc.record({
              dx: fc.integer({ max: 2, min: -2 }),
              dy: fc.integer({ max: 2, min: -2 }),
              tile: fc.constantFrom("a1", "a2"),
            }),
            { maxLength: 6 }
          ),
          (presses) =>
            Effect.runPromise(
              play(
                Array.flatMap(presses, ({ dx, dy, tile }): Step[] => [
                  {
                    event: {
                      clientX: 100,
                      clientY: 100,
                      key: tile,
                      type: "tile.press",
                    },
                    kind: "event",
                  },
                  {
                    event: {
                      clientX: 100 + dx,
                      clientY: 100 + dy,
                      type: "pointer.move",
                    },
                    kind: "event",
                  },
                  { event: { type: "pointer.release" }, kind: "event" },
                  { event: { type: "workspace.close" }, kind: "event" },
                ]),
                []
              ).pipe(
                Effect.tap(({ sent }) =>
                  Effect.sync(() => {
                    assert.deepStrictEqual(
                      Array.filter(sent, (placement) => placement.id !== 3),
                      []
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
});
