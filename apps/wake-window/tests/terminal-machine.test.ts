import { assert, describe, it } from "@effect/vitest";
import { Array, Effect } from "effect";

import {
  cloud,
  daemonGoes,
  drained,
  forget,
  local,
  screenOf,
  shellExits,
  shellSays,
  terminalHarness,
  waitRunsOut,
} from "./support/terminal-harness.ts";
import type { TerminalWorld } from "./support/terminal-harness.ts";

const DEBOUNCE = 150;

const shown = (world: TerminalWorld): Effect.Effect<TerminalWorld> =>
  drained(world).pipe(
    Effect.tap(() =>
      Effect.sync(() => {
        world.actor.send({
          cols: 80,
          key: "a1",
          rows: 24,
          type: "terminal.show",
        });
      })
    ),
    Effect.flatMap(() => drained(world)),
    Effect.as(world)
  );

describe("the terminal region", () => {
  it.effect(
    "opens the agent's shell at the pane's size when the pane shows",
    () =>
      Effect.gen(function* opens() {
        const world = terminalHarness([local(1)]);
        const before = yield* drained(world);
        assert.isTrue(before.matches({ terminal: "closed" }));
        yield* shown(world);
        const snapshot = world.actor.getSnapshot();
        assert.isTrue(snapshot.matches({ terminal: "open" }));
        assert.deepStrictEqual(world.opened, [
          { cols: 80, id: 1, location: "local", rows: 24 },
        ]);
        assert.deepStrictEqual(world.feed, [{ type: "terminal.reset" }]);
        assert.deepStrictEqual(world.reads, [
          { after: 0, terminal: 1, wait_ms: 10_000 },
        ]);
      })
  );

  it.effect(
    "feeds output in order and asks for the rest from the last offset",
    () =>
      Effect.gen(function* reads() {
        const world = yield* shown(terminalHarness([local(1)]));
        shellSays({ text: "$ ", world });
        yield* drained(world);
        shellSays({ text: "hello\r\n", world });
        const snapshot = yield* drained(world);
        assert.strictEqual(screenOf(world.feed), "$ hello\r\n");
        assert.strictEqual(snapshot.context.terminal.next, 9);
        assert.deepStrictEqual(
          Array.map(world.reads, ({ after }) => after),
          [0, 2, 9]
        );
        assert.isFalse(world.overlapped);
      })
  );

  it.effect("asks again from the same offset when a wait runs out", () =>
    Effect.gen(function* waits() {
      const world = yield* shown(terminalHarness([local(1)]));
      waitRunsOut(world);
      const snapshot = yield* drained(world);
      assert.isTrue(snapshot.matches({ terminal: "open" }));
      assert.deepStrictEqual(
        Array.map(world.reads, ({ after }) => after),
        [0, 0]
      );
    })
  );

  it.effect(
    "sends keystrokes in order, joining those typed while one is on its way",
    () =>
      Effect.gen(function* types() {
        const world = yield* shown(terminalHarness([local(1)]));
        world.actor.send({ data: "l", type: "terminal.type" });
        world.actor.send({ data: "s", type: "terminal.type" });
        world.actor.send({ data: "\r", type: "terminal.type" });
        yield* drained(world);
        assert.deepStrictEqual(world.written, [
          { data: "l", whileOpen: true },
          { data: "s\r", whileOpen: true },
        ]);
        assert.strictEqual(screenOf(world.feed), "ls\r");
      })
  );

  it.effect("No write is sent before open succeeded or after exit", () =>
    Effect.gen(function* guarded() {
      const world = terminalHarness([local(1)]);
      yield* drained(world);
      world.actor.send({ data: "early", type: "terminal.type" });
      yield* shown(world);
      shellExits({ code: 3, world });
      const exited = yield* drained(world);
      world.actor.send({ data: "late", type: "terminal.type" });
      yield* drained(world);
      assert.isTrue(exited.matches({ terminal: "exited" }));
      assert.strictEqual(exited.context.terminal.exitCode, 3);
      assert.deepStrictEqual(world.written, []);
    })
  );

  it.effect(
    "starts a new shell after an exit and clears the screen for it",
    () =>
      Effect.gen(function* restarts() {
        const world = yield* shown(terminalHarness([local(1)]));
        shellSays({ text: "old", world });
        shellExits({ code: 0, world });
        yield* drained(world);
        world.actor.send({ type: "terminal.restart" });
        forget(world);
        yield* drained(world);
        shellSays({ text: "new", world });
        const snapshot = yield* drained(world);
        assert.isTrue(snapshot.matches({ terminal: "open" }));
        assert.strictEqual(snapshot.context.terminal.id, 2);
        assert.strictEqual(snapshot.context.terminal.exitCode, null);
        assert.strictEqual(screenOf(world.feed), "new");
      })
  );

  it.effect("Leaving the pane stops reading", () =>
    Effect.gen(function* leaves() {
      const world = yield* shown(terminalHarness([local(1)]));
      shellSays({ text: "one", world });
      yield* drained(world);
      world.actor.send({ type: "terminal.hide" });
      forget(world);
      const hidden = yield* drained(world);
      const readsWhenHidden = world.reads.length;
      const feedWhenHidden = world.feed.length;
      shellSays({ text: "two", world });
      yield* drained(world);
      assert.isTrue(hidden.matches({ terminal: "closed" }));
      assert.lengthOf(world.reads, readsWhenHidden);
      assert.lengthOf(world.feed, feedWhenHidden);
    })
  );

  it.effect(
    "replays the shell's output from the start when the pane returns",
    () =>
      Effect.gen(function* returns() {
        const world = yield* shown(terminalHarness([local(1)]));
        shellSays({ text: "one", world });
        yield* drained(world);
        world.actor.send({ type: "terminal.hide" });
        forget(world);
        yield* drained(world);
        shellSays({ text: "two", world });
        yield* shown(world);
        const snapshot = world.actor.getSnapshot();
        assert.strictEqual(snapshot.context.terminal.id, 1);
        assert.strictEqual(screenOf(world.feed), "onetwo");
        assert.isFalse(world.overlapped);
      })
  );

  it.effect(
    "sends one resize for a burst of fits, and none for the size it has",
    () =>
      Effect.gen(function* resizes() {
        const world = yield* shown(terminalHarness([local(1)]));
        world.actor.send({ cols: 90, rows: 24, type: "terminal.fit" });
        world.clock.increment(DEBOUNCE - 50);
        world.actor.send({ cols: 100, rows: 30, type: "terminal.fit" });
        world.clock.increment(DEBOUNCE - 50);
        yield* drained(world);
        const early = world.resized.length;
        world.clock.increment(DEBOUNCE);
        yield* drained(world);
        world.actor.send({ cols: 100, rows: 30, type: "terminal.fit" });
        world.clock.increment(DEBOUNCE * 2);
        yield* drained(world);
        assert.strictEqual(early, 0);
        assert.deepStrictEqual(world.resized, [
          { cols: 100, rows: 30, terminal: 1 },
        ]);
      })
  );

  it.effect(
    "fails with the reason when the daemon goes away, and picks up where it was",
    () =>
      Effect.gen(function* fails() {
        const world = yield* shown(terminalHarness([local(1)]));
        shellSays({ text: "before ", world });
        yield* drained(world);
        daemonGoes({ down: true, world });
        const down = yield* drained(world);
        daemonGoes({ down: false, world });
        shellSays({ text: "after", world });
        world.actor.send({ type: "terminal.restart" });
        forget(world);
        const back = yield* drained(world);
        assert.isTrue(down.matches({ terminal: "failed" }));
        assert.deepStrictEqual(down.context.terminal.problem, {
          kind: "daemon_down",
          reason: "connect ENOENT /wake.sock",
        });
        assert.isTrue(back.matches({ terminal: "open" }));
        assert.strictEqual(screenOf(world.feed), "before after");
        assert.lengthOf(
          Array.filter(world.feed, (entry) => entry.type === "terminal.reset"),
          1
        );
      })
  );

  it.effect("fails with the daemon's reason when the open is refused", () =>
    Effect.gen(function* refused() {
      const world = terminalHarness([local(1)]);
      daemonGoes({ down: true, world });
      yield* shown(world);
      const snapshot = world.actor.getSnapshot();
      assert.isTrue(snapshot.matches({ terminal: "failed" }));
      assert.strictEqual(snapshot.context.terminal.problem.kind, "daemon_down");
      assert.deepStrictEqual(world.reads, []);
    })
  );

  it.effect("opens nothing for a cloud task or an agent it does not know", () =>
    Effect.gen(function* cloudTask() {
      const world = terminalHarness([cloud(1)]);
      yield* drained(world);
      world.actor.send({
        cols: 80,
        key: "c1",
        rows: 24,
        type: "terminal.show",
      });
      world.actor.send({
        cols: 80,
        key: "a9",
        rows: 24,
        type: "terminal.show",
      });
      const snapshot = yield* drained(world);
      assert.isTrue(snapshot.matches({ terminal: "closed" }));
      assert.deepStrictEqual(world.opened, []);
    })
  );

  it.effect("closes when the workspace closes", () =>
    Effect.gen(function* closes() {
      const world = yield* shown(terminalHarness([local(1)]));
      world.actor.send({ type: "workspace.close" });
      forget(world);
      const snapshot = yield* drained(world);
      assert.isTrue(snapshot.matches({ terminal: "closed" }));
      assert.strictEqual(snapshot.context.terminal.key, "");
    })
  );

  it.effect("closes when its agent leaves the fleet", () =>
    Effect.gen(function* agentLeaves() {
      const world = yield* shown(terminalHarness([local(1)]));
      world.agents.length = 0;
      world.clock.increment(2000);
      const snapshot = yield* drained(world);
      assert.isTrue(snapshot.matches({ terminal: "closed" }));
    })
  );
});
