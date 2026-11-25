import { Array, Effect, Option } from "effect";
import { SimulatedClock, createActor, fromPromise } from "xstate";
import type { Actor } from "xstate";

import type { TerminalFeed } from "#/renderer/window/terminal-context-model.ts";
import type { FleetSnapshot } from "#/renderer/window/window-context-model.ts";
import { windowMachine } from "#/renderer/window/window-machine.ts";
import type { WindowSnapshot } from "#/renderer/window/window-machine.ts";
import type { Agent } from "#/shared/agent.ts";
import type {
  Input,
  OpenOutcome,
  Opening,
  ReadOutcome,
  Reading,
  Resize,
  SentOutcome,
} from "#/shared/terminal.ts";

type WindowActor = Actor<typeof windowMachine>;

interface Shell {
  bytes: number[];
  exited: number | null;
  readonly id: number;
}

interface Waiting {
  readonly reading: Reading;
  readonly resolve: (outcome: ReadOutcome) => void;
}

interface Written {
  readonly data: string;
  readonly whileOpen: boolean;
}

interface TerminalWorld {
  readonly actor: WindowActor;
  readonly agents: Agent[];
  readonly clock: SimulatedClock;
  down: boolean;
  readonly feed: TerminalFeed[];
  readonly opened: Opening[];
  overlapped: boolean;
  readonly reads: Reading[];
  readonly resized: Resize[];
  readonly shells: Shell[];
  readonly waiting: Waiting[];
  readonly written: Written[];
}

const HOPS = 60;
const DOWN: { readonly kind: "daemon_down"; readonly reason: string } = {
  kind: "daemon_down",
  reason: "connect ENOENT /wake.sock",
};
const encoder = new TextEncoder();
const decoder = new TextDecoder();

const local = (id: number): Agent => ({
  branch: null,
  folder: `/work/${id}`,
  id,
  location: "local",
  position: { x: 40 + id * 320, y: 40 },
  progress: { state: "missing" },
  started_at: id,
});

const cloud = (id: number): Agent => ({
  ...local(id),
  branch: "claude/task",
  location: "cloud",
});

const liveShell = (world: TerminalWorld): Option.Option<Shell> =>
  Option.filter(Array.last(world.shells), (shell) => shell.exited === null);

const shellOf = (world: TerminalWorld, id: number): Option.Option<Shell> =>
  Array.findFirst(world.shells, (shell) => shell.id === id);

const answerOf = (shell: Shell, reading: Reading): ReadOutcome => ({
  bytes: Uint8Array.from(Array.drop(shell.bytes, reading.after)),
  exited: shell.exited,
  kind: "output",
  next: shell.bytes.length,
  truncated: false,
});

const settleWaiting = (world: TerminalWorld, force: boolean): void => {
  const held = world.waiting.splice(0);
  for (const waiting of held) {
    const outcome = Option.match(shellOf(world, waiting.reading.terminal), {
      onNone: (): ReadOutcome => ({
        kind: "refused",
        reason: `there is no terminal with ID ${waiting.reading.terminal}`,
      }),
      onSome: (shell): ReadOutcome => answerOf(shell, waiting.reading),
    });
    const ready =
      force ||
      world.down ||
      outcome.kind !== "output" ||
      outcome.exited !== null ||
      outcome.bytes.length > 0;
    if (ready) {
      waiting.resolve(world.down ? DOWN : outcome);
    } else {
      world.waiting.push(waiting);
    }
  }
};

const openFor = (world: TerminalWorld, opening: Opening): OpenOutcome => {
  world.opened.push(opening);
  if (world.down) {
    return DOWN;
  }
  if (opening.location === "cloud") {
    return { kind: "refused", reason: "a cloud task has no terminal" };
  }
  return Option.match(liveShell(world), {
    onNone: (): OpenOutcome => {
      const shell: Shell = {
        bytes: [],
        exited: null,
        id: world.shells.length + 1,
      };
      world.shells.push(shell);
      return { kind: "opened", terminal: shell.id };
    },
    onSome: ({ id }): OpenOutcome => ({ kind: "opened", terminal: id }),
  });
};

const writeFor = (world: TerminalWorld, input: Input): SentOutcome => {
  world.written.push({
    data: input.data,
    whileOpen: world.actor.getSnapshot().matches({ terminal: "open" }),
  });
  if (world.down) {
    return DOWN;
  }
  return Option.match(
    Option.filter(
      shellOf(world, input.terminal),
      (shell) => shell.exited === null
    ),
    {
      onNone: (): SentOutcome => ({ kind: "refused", reason: "exited" }),
      onSome: (shell): SentOutcome => {
        shell.bytes.push(...encoder.encode(input.data));
        settleWaiting(world, false);
        return { kind: "sent" };
      },
    }
  );
};

const readFor = (
  world: TerminalWorld,
  reading: Reading
): Promise<ReadOutcome> => {
  world.reads.push(reading);
  if (
    Array.some(
      world.waiting,
      (waiting) => waiting.reading.terminal === reading.terminal
    )
  ) {
    world.overlapped = true;
  }
  const { promise, resolve } = Promise.withResolvers<ReadOutcome>();
  world.waiting.push({ reading, resolve });
  settleWaiting(world, false);
  return promise;
};

const forget = (world: TerminalWorld): void => {
  settleWaiting(world, true);
};

const terminalHarness = (agents: readonly Agent[]): TerminalWorld => {
  const clock = new SimulatedClock();
  const seeded = [...agents];
  const world: { current: TerminalWorld | null } = { current: null };
  const worldNow = (): TerminalWorld => {
    if (world.current === null) {
      throw new Error("the terminal harness is used before it is built");
    }
    return world.current;
  };
  const actor = createActor(
    windowMachine.provide({
      actors: {
        openTerminal: fromPromise(({ input }) =>
          Promise.resolve(openFor(worldNow(), input))
        ),
        readFleet: fromPromise(() =>
          Promise.resolve<FleetSnapshot>({
            at: clock.now(),
            reading: { agents: [...seeded], kind: "agents" },
          })
        ),
        readTerminal: fromPromise(({ input }) => readFor(worldNow(), input)),
        resizeTerminal: fromPromise(({ input }) => {
          worldNow().resized.push(input);
          return Promise.resolve<SentOutcome>({ kind: "sent" });
        }),
        writeTerminal: fromPromise(({ input }) =>
          Promise.resolve(writeFor(worldNow(), input))
        ),
      },
    }),
    { clock }
  );
  const built: TerminalWorld = {
    actor,
    agents: seeded,
    clock,
    down: false,
    feed: [],
    opened: [],
    overlapped: false,
    reads: [],
    resized: [],
    shells: [],
    waiting: [],
    written: [],
  };
  world.current = built;
  actor.on("terminal.output", (feed) => {
    built.feed.push(feed);
  });
  actor.on("terminal.reset", (feed) => {
    built.feed.push(feed);
  });
  actor.start();
  return built;
};

const drained = (world: TerminalWorld): Effect.Effect<WindowSnapshot> =>
  Effect.promise(() => Promise.resolve()).pipe(
    Effect.repeat({ times: HOPS }),
    Effect.map(() => world.actor.getSnapshot())
  );

interface Says {
  readonly text: string;
  readonly world: TerminalWorld;
}

interface Exits {
  readonly code: number;
  readonly world: TerminalWorld;
}

interface Goes {
  readonly down: boolean;
  readonly world: TerminalWorld;
}

const shellSays = ({ text, world }: Says): void => {
  Option.match(liveShell(world), {
    onNone: () => null,
    onSome: (shell) => {
      shell.bytes.push(...encoder.encode(text));
      return null;
    },
  });
  settleWaiting(world, false);
};

const shellExits = ({ code, world }: Exits): void => {
  Option.match(liveShell(world), {
    onNone: () => null,
    onSome: (shell) => {
      shell.exited = code;
      return null;
    },
  });
  settleWaiting(world, false);
};

const daemonGoes = ({ down, world }: Goes): void => {
  world.down = down;
  settleWaiting(world, false);
};

const waitRunsOut = (world: TerminalWorld): void => {
  settleWaiting(world, true);
};

const screenOf = (feed: readonly TerminalFeed[]): string => {
  const shown: number[] = [];
  for (const entry of feed) {
    if (entry.type === "terminal.reset") {
      shown.length = 0;
    } else {
      shown.push(...entry.bytes);
    }
  }
  return decoder.decode(Uint8Array.from(shown));
};

const textOf = (shell: Shell): string =>
  decoder.decode(Uint8Array.from(shell.bytes));

export {
  cloud,
  daemonGoes,
  drained,
  forget,
  liveShell,
  local,
  screenOf,
  shellExits,
  shellSays,
  terminalHarness,
  textOf,
  waitRunsOut,
};
export type { Shell, TerminalWorld };
