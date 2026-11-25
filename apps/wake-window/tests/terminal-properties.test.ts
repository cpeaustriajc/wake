import { assert, describe, it } from "@effect/vitest";
import { Array, Effect, Option, String } from "effect";
import * as fc from "fast-check";

import type { WindowSnapshot } from "#/renderer/window/window-machine.ts";

import {
  daemonGoes,
  drained,
  forget,
  local,
  screenOf,
  shellExits,
  shellSays,
  terminalHarness,
  textOf,
  waitRunsOut,
} from "./support/terminal-harness.ts";
import type { TerminalWorld } from "./support/terminal-harness.ts";

type Step =
  | { readonly code: number; readonly kind: "exits" }
  | { readonly cols: number; readonly kind: "fit"; readonly rows: number }
  | { readonly down: boolean; readonly kind: "daemon" }
  | { readonly kind: "close" | "hide" | "restart" | "runsOut" | "show" }
  | { readonly kind: "says"; readonly text: string }
  | { readonly kind: "tick"; readonly milliseconds: number }
  | { readonly kind: "type" };

const SETTLE_MILLISECONDS = 1000;

const step: fc.Arbitrary<Step> = fc.oneof(
  { arbitrary: fc.constant<Step>({ kind: "show" }), weight: 4 },
  { arbitrary: fc.constant<Step>({ kind: "type" }), weight: 5 },
  {
    arbitrary: fc.record({
      kind: fc.constant("says" as const),
      text: fc.string({ maxLength: 6, minLength: 1 }),
    }),
    weight: 5,
  },
  { arbitrary: fc.constant<Step>({ kind: "runsOut" }), weight: 2 },
  {
    arbitrary: fc.constantFrom<Step>(
      { kind: "hide" },
      { kind: "close" },
      { kind: "restart" }
    ),
    weight: 3,
  },
  {
    arbitrary: fc.record({
      code: fc.integer({ max: 3, min: 0 }),
      kind: fc.constant("exits" as const),
    }),
    weight: 1,
  },
  {
    arbitrary: fc.record({
      down: fc.boolean(),
      kind: fc.constant("daemon" as const),
    }),
    weight: 1,
  },
  {
    arbitrary: fc.record({
      cols: fc.integer({ max: 120, min: 20 }),
      kind: fc.constant("fit" as const),
      rows: fc.integer({ max: 50, min: 5 }),
    }),
    weight: 2,
  },
  {
    arbitrary: fc.record({
      kind: fc.constant("tick" as const),
      milliseconds: fc.integer({ max: 400, min: 10 }),
    }),
    weight: 2,
  }
);

const tokensIn = (written: string): readonly number[] =>
  Array.map(
    Array.filter(String.split(written, ";"), String.startsWith("k")),
    (token) => Number(String.slice(1)(token))
  );

const increasing = (numbers: readonly number[]): boolean =>
  Array.every(
    Array.zip(numbers, Array.drop(numbers, 1)),
    ([earlier, later]) => earlier < later
  );

const holds = (world: TerminalWorld, snapshot: WindowSnapshot): void => {
  const { terminal } = snapshot.context;
  const open = snapshot.matches({ terminal: "open" });
  assert.isFalse(world.overlapped, "Never two read loops for one terminal");
  assert.lengthOf(
    world.waiting,
    open ? 1 : 0,
    "One read waits while the terminal is open, and leaving the pane stops reading"
  );
  assert.deepStrictEqual(
    Array.filter(world.written, ({ whileOpen }) => !whileOpen),
    [],
    "No write is sent before open succeeded or after exit"
  );
  assert.isTrue(
    increasing(
      tokensIn(
        Array.join(
          Array.map(world.written, ({ data }) => data),
          ""
        )
      )
    ),
    "Keystrokes reach the shell in the order they were typed, each once"
  );
  const shell = Array.findFirst(
    world.shells,
    ({ id }) => !snapshot.matches({ terminal: "closed" }) && id === terminal.id
  );
  if (Option.isSome(shell) && !snapshot.matches({ terminal: "opening" })) {
    assert.strictEqual(
      screenOf(world.feed),
      String.slice(0, screenOf(world.feed).length)(textOf(shell.value)),
      "Bytes are appended in offset order with no gap or duplicate"
    );
    assert.strictEqual(
      new TextEncoder().encode(screenOf(world.feed)).length <=
        shell.value.bytes.length,
      true,
      "The screen never shows more than the shell wrote"
    );
  }
};

const apply = (world: TerminalWorld, next: Step, index: number): void => {
  if (next.kind === "show") {
    world.actor.send({ cols: 80, key: "a1", rows: 24, type: "terminal.show" });
    forget(world);
  } else if (next.kind === "hide") {
    world.actor.send({ type: "terminal.hide" });
    forget(world);
  } else if (next.kind === "close") {
    world.actor.send({ type: "workspace.close" });
    forget(world);
  } else if (next.kind === "restart") {
    world.actor.send({ type: "terminal.restart" });
    forget(world);
  } else if (next.kind === "type") {
    world.actor.send({ data: `k${index};`, type: "terminal.type" });
  } else if (next.kind === "fit") {
    world.actor.send({
      cols: next.cols,
      rows: next.rows,
      type: "terminal.fit",
    });
  } else if (next.kind === "says") {
    shellSays({ text: next.text, world });
  } else if (next.kind === "exits") {
    shellExits({ code: next.code, world });
  } else if (next.kind === "tick") {
    world.clock.increment(next.milliseconds);
  } else if (next.kind === "daemon") {
    daemonGoes({ down: next.down, world });
  } else {
    waitRunsOut(world);
  }
};

const playOn = (
  world: TerminalWorld,
  steps: readonly Step[]
): Effect.Effect<TerminalWorld> =>
  Effect.forEach(steps, (next, index) =>
    Effect.sync(() => {
      apply(world, next, index);
    }).pipe(
      Effect.flatMap(() => drained(world)),
      Effect.map((snapshot) => {
        holds(world, snapshot);
        return snapshot;
      })
    )
  ).pipe(Effect.as(world));

const play = (steps: readonly Step[]): Effect.Effect<TerminalWorld> => {
  const world = terminalHarness([local(1)]);
  return drained(world).pipe(Effect.flatMap(() => playOn(world, steps)));
};

interface Cells {
  readonly cols: number;
  readonly rows: number;
}

const SHOWN: Cells = { cols: 80, rows: 24 };

const cellsOf = ({ cols, rows }: Cells): Cells => ({ cols, rows });

const stepsOf = fc.array(step, { maxLength: 40 });

const fitsOf = fc.array(
  fc.oneof(
    fc.record({
      cols: fc.integer({ max: 120, min: 20 }),
      kind: fc.constant("fit" as const),
      rows: fc.integer({ max: 50, min: 5 }),
    }),
    fc.record({
      kind: fc.constant("tick" as const),
      milliseconds: fc.integer({ max: 400, min: 10 }),
    })
  ),
  { maxLength: 8 }
);

describe("the terminal region, under any sequence of events", () => {
  it.effect(
    "keeps its promises about read loops, byte order, writes and leaving the pane",
    () =>
      Effect.promise(() =>
        fc.assert(
          fc.asyncProperty(stepsOf, (steps) =>
            play(steps).pipe(Effect.asVoid, Effect.runPromise)
          )
        )
      )
  );

  it.effect(
    "The screen shows exactly what the shell wrote once the terminal is open and quiet",
    () =>
      Effect.promise(() =>
        fc.assert(
          fc.asyncProperty(stepsOf, (steps) =>
            play([
              ...steps,
              { kind: "daemon", down: false },
              { kind: "show" },
            ]).pipe(
              Effect.flatMap((world) =>
                drained(world).pipe(
                  Effect.map((snapshot) => {
                    const shell = Array.findFirst(
                      world.shells,
                      ({ id }) => id === snapshot.context.terminal.id
                    );
                    assert.isTrue(
                      snapshot.matches({ terminal: "open" }) ||
                        snapshot.matches({ terminal: "exited" })
                    );
                    assert.strictEqual(
                      screenOf(world.feed),
                      Option.match(shell, {
                        onNone: () => "no shell",
                        onSome: textOf,
                      })
                    );
                    return snapshot;
                  })
                )
              ),
              Effect.asVoid,
              Effect.runPromise
            )
          )
        )
      )
  );

  it.effect(
    "The shell's size follows the pane once the pane stops changing",
    () =>
      Effect.promise(() =>
        fc.assert(
          fc.asyncProperty(stepsOf, fitsOf, (steps, fits) =>
            play([
              ...steps,
              { kind: "daemon", down: false },
              { kind: "show" },
            ]).pipe(
              Effect.flatMap((world) => {
                const before = world.resized.length;
                return playOn(world, fits).pipe(
                  Effect.tap(() =>
                    Effect.sync(() => {
                      world.clock.increment(SETTLE_MILLISECONDS);
                    })
                  ),
                  Effect.flatMap(() => drained(world)),
                  Effect.map((snapshot) => {
                    const { sentSize, size } = snapshot.context.terminal;
                    const wanted = Option.getOrElse(
                      Array.findLast(fits, (fit) => fit.kind === "fit"),
                      () => SHOWN
                    );
                    const told = Option.getOrElse(
                      Array.last(Array.drop(world.resized, before)),
                      () => SHOWN
                    );
                    assert.deepStrictEqual(size, cellsOf(wanted));
                    assert.deepStrictEqual(sentSize, size);
                    assert.deepStrictEqual(cellsOf(told), size);
                    return snapshot;
                  })
                );
              }),
              Effect.asVoid,
              Effect.runPromise
            )
          )
        )
      )
  );

  it.effect("A cloud task or an unknown agent never opens a terminal", () =>
    Effect.promise(() =>
      fc.assert(
        fc.asyncProperty(
          fc.array(fc.constantFrom("c1", "a9", ""), { maxLength: 6 }),
          (keys) =>
            Effect.runPromise(
              Effect.sync(() => terminalHarness([local(1)])).pipe(
                Effect.tap(drained),
                Effect.tap((world) =>
                  Effect.sync(() => {
                    for (const key of keys) {
                      world.actor.send({
                        cols: 80,
                        key,
                        rows: 24,
                        type: "terminal.show",
                      });
                    }
                  })
                ),
                Effect.flatMap((world) =>
                  drained(world).pipe(
                    Effect.map((snapshot) => {
                      assert.isTrue(snapshot.matches({ terminal: "closed" }));
                      assert.deepStrictEqual(world.opened, []);
                      return snapshot;
                    })
                  )
                ),
                Effect.asVoid
              )
            )
        )
      )
    )
  );
});
