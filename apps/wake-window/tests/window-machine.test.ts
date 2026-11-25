import { assert, describe, it } from "@effect/vitest";
import { Effect } from "effect";

import { layoutIn } from "#/renderer/window/window-context-model.ts";

import { agent, harness, live, poll, quiet } from "./support/window-harness.ts";

describe("the fleet region", () => {
  it.effect("goes from connecting to live on a good read", () =>
    Effect.gen(function* fleetLive() {
      const { actor } = harness({
        readings: [live(agent({ id: 1, position: { x: 40, y: 40 } }))],
      });
      assert.isTrue(
        actor.getSnapshot().matches({ fleet: { status: "connecting" } })
      );
      const snapshot = yield* quiet(actor);
      assert.isTrue(snapshot.matches({ fleet: { status: "live" } }));
      assert.lengthOf(snapshot.context.agents, 1);
    })
  );

  it.effect(
    "keeps the last agents when a later read fails, then recovers",
    () =>
      Effect.gen(function* fleetDown() {
        const harnessed = harness({
          readings: [
            live(agent({ id: 1, position: { x: 40, y: 40 } })),
            { kind: "daemon_down", reason: "refused", socket: "/s" },
            live(
              agent({ id: 1, position: { x: 40, y: 40 } }),
              agent({ id: 2, position: { x: 360, y: 40 } })
            ),
          ],
        });
        yield* quiet(harnessed.actor);
        const down = yield* poll(harnessed);
        assert.isTrue(down.matches({ fleet: { status: "daemonDown" } }));
        assert.lengthOf(down.context.agents, 1);
        const back = yield* poll(harnessed);
        assert.isTrue(back.matches({ fleet: { status: "live" } }));
        assert.lengthOf(back.context.agents, 2);
      })
  );
});

describe("the placement region", () => {
  it.effect("opens the workspace on a press that never moves 5 px", () =>
    Effect.gen(function* click() {
      const { actor, sent } = harness({
        readings: [live(agent({ id: 1, position: { x: 40, y: 40 } }))],
      });
      yield* quiet(actor);
      actor.send({ clientX: 100, clientY: 100, key: "a1", type: "tile.press" });
      actor.send({ clientX: 102, clientY: 101, type: "pointer.move" });
      actor.send({ type: "pointer.release" });
      actor.send({ key: "a1", type: "tile.click" });
      const snapshot = yield* quiet(actor);
      assert.isTrue(snapshot.matches({ navigation: "workspace" }));
      assert.strictEqual(snapshot.context.openedKey, "a1");
      assert.lengthOf(sent, 0);
    })
  );

  it.effect("drags after 5 px, snaps, saves once and swallows the click", () =>
    Effect.gen(function* drag() {
      const { actor, sent } = harness({
        readings: [live(agent({ id: 1, position: { x: 40, y: 40 } }))],
      });
      yield* quiet(actor);
      actor.send({ clientX: 100, clientY: 100, key: "a1", type: "tile.press" });
      actor.send({ clientX: 409, clientY: 228, type: "pointer.move" });
      assert.isTrue(
        actor.getSnapshot().matches({ placement: { gesture: "dragging" } })
      );
      actor.send({ type: "pointer.release" });
      actor.send({ key: "a1", type: "tile.click" });
      const snapshot = yield* quiet(actor);
      assert.deepStrictEqual(sent, [
        { id: 1, location: "local", x: 340, y: 160 },
      ]);
      assert.isTrue(snapshot.matches({ navigation: "floor" }));
      assert.deepStrictEqual(layoutIn(snapshot.context), {
        a1: { x: 340, y: 160 },
      });
    })
  );

  it.effect(
    "keeps the dropped position through a stale poll and lets go when the daemon agrees",
    () =>
      Effect.gen(function* pending() {
        const harnessed = harness({
          readings: [
            live(agent({ id: 1, position: { x: 40, y: 40 } })),
            live(agent({ id: 1, position: { x: 40, y: 40 } })),
            live(agent({ id: 1, position: { x: 340, y: 160 } })),
          ],
        });
        const { actor } = harnessed;
        yield* quiet(actor);
        actor.send({
          clientX: 100,
          clientY: 100,
          key: "a1",
          type: "tile.press",
        });
        actor.send({ clientX: 400, clientY: 220, type: "pointer.move" });
        actor.send({ type: "pointer.release" });
        const stale = yield* poll(harnessed);
        assert.deepStrictEqual(stale.context.held, { a1: { x: 340, y: 160 } });
        const agreed = yield* poll(harnessed);
        assert.deepStrictEqual(agreed.context.held, {});
        assert.deepStrictEqual(layoutIn(agreed.context), {
          a1: { x: 340, y: 160 },
        });
      })
  );

  it.effect(
    "returns the tile and shows the reason when the daemon refuses",
    () =>
      Effect.gen(function* refused() {
        const { actor } = harness({
          outcomes: [
            { kind: "refused", reason: "there is no local agent with ID 1" },
          ],
          readings: [live(agent({ id: 1, position: { x: 40, y: 40 } }))],
        });
        yield* quiet(actor);
        actor.send({ key: "a1", stepsX: 1, stepsY: 0, type: "tile.nudge" });
        const snapshot = yield* quiet(actor);
        assert.isTrue(
          snapshot.matches({ placement: { save: "couldNotPlace" } })
        );
        assert.deepStrictEqual(snapshot.context.failures, [
          { key: "a1", reason: "there is no local agent with ID 1" },
        ]);
        assert.deepStrictEqual(layoutIn(snapshot.context), {
          a1: { x: 40, y: 40 },
        });
      })
  );

  it.effect("sends an unplaced agent to the first free slot exactly once", () =>
    Effect.gen(function* once() {
      const harnessed = harness({
        readings: [
          live(agent({ id: 1, position: { x: 40, y: 40 } }), agent({ id: 2 })),
        ],
      });
      yield* quiet(harnessed.actor);
      yield* poll(harnessed);
      const snapshot = yield* poll(harnessed);
      assert.deepStrictEqual(harnessed.sent, [
        { id: 2, location: "local", x: 360, y: 40 },
      ]);
      assert.deepStrictEqual(layoutIn(snapshot.context), {
        a1: { x: 40, y: 40 },
        a2: { x: 360, y: 40 },
      });
    })
  );
});

describe("the navigation region", () => {
  it.effect("leaves the workspace when its agent leaves the fleet", () =>
    Effect.gen(function* leaves() {
      const harnessed = harness({
        readings: [live(agent({ id: 1, position: { x: 40, y: 40 } })), live()],
      });
      const { actor } = harnessed;
      yield* quiet(actor);
      actor.send({ key: "a1", type: "tile.click" });
      assert.isTrue(actor.getSnapshot().matches({ navigation: "workspace" }));
      const snapshot = yield* poll(harnessed);
      assert.isTrue(snapshot.matches({ navigation: "floor" }));
      assert.strictEqual(snapshot.context.openedKey, "");
    })
  );
});
