import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";

import { assert, describe, it } from "@effect/vitest";
import { Effect, Schedule, String } from "effect";

import { askDaemon } from "#/main/daemon-client.ts";
import {
  listAgents,
  openTerminal,
  readTerminal,
  resizeTerminal,
  writeTerminal,
} from "#/main/daemon-pipe.ts";
import {
  decodeOpenOutcome,
  decodeReadOutcome,
  decodeSentOutcome,
} from "#/shared/terminal.ts";
import type { ReadOutcome } from "#/shared/terminal.ts";

const BINARY = new URL("../../../target/debug/wake", import.meta.url).pathname;
const WAIT_MILLISECONDS = 300;
const TRIES = 60;

interface Seen {
  readonly exited: number | null;
  readonly next: number;
  readonly text: string;
  readonly tries: number;
  readonly trouble: string;
}

interface Watch {
  readonly from: number;
  readonly socketPath: string;
  readonly terminal: number;
  readonly until: (seen: Seen) => boolean;
}

const decoder = new TextDecoder();

const scratch = (): string => mkdtempSync(`${tmpdir()}/wake-terminal-`);

const runningDaemon = Effect.acquireRelease(
  Effect.sync(() => {
    const folder = scratch();
    const socketPath = `${folder}/d.sock`;
    const daemon = spawn(BINARY, ["daemon"], {
      env: {
        HOME: folder,
        PATH: "/usr/bin:/bin",
        SHELL: "/bin/sh",
        WAKE_DB: `${folder}/d.db`,
        WAKE_SOCKET: socketPath,
      },
      stdio: "ignore",
    });
    return { daemon, folder, socketPath };
  }),
  ({ daemon }) =>
    Effect.sync(() => {
      daemon.kill();
    })
).pipe(
  Effect.tap(({ socketPath }) =>
    listAgents(socketPath).pipe(
      Effect.filterOrFail((answer) => answer.kind === "answered"),
      Effect.retry({ schedule: Schedule.spaced("50 millis"), times: 100 })
    )
  )
);

const agentIn = (folder: string, socketPath: string) => {
  const path = `${folder}/agent-one`;
  mkdirSync(path, { recursive: true });
  return askDaemon({
    method: "agent.register",
    params: { folder: path },
    socketPath,
  });
};

const merged = (seen: Seen, outcome: ReadOutcome): Seen =>
  outcome.kind === "output"
    ? {
        exited: outcome.exited,
        next: outcome.next,
        text: `${seen.text}${decoder.decode(outcome.bytes)}`,
        tries: seen.tries + 1,
        trouble: "",
      }
    : { ...seen, tries: TRIES, trouble: outcome.reason };

const watched = ({
  from,
  socketPath,
  terminal,
  until,
}: Watch): Effect.Effect<Seen> => {
  const step = (seen: Seen): Effect.Effect<Seen> =>
    until(seen) || seen.tries >= TRIES
      ? Effect.succeed(seen)
      : readTerminal({
          socketPath,
          wire: { after: seen.next, terminal, wait_ms: WAIT_MILLISECONDS },
        }).pipe(
          Effect.flatMap(decodeReadOutcome),
          Effect.flatMap((outcome) => step(merged(seen, outcome)))
        );
  return step({ exited: null, next: from, text: "", tries: 0, trouble: "" });
};

const typed = (socketPath: string, terminal: number, data: string) =>
  writeTerminal({ socketPath, wire: { data, terminal } }).pipe(
    Effect.flatMap(decodeSentOutcome)
  );

describe("the terminal pipe against the real daemon and a real shell", () => {
  it.live(
    "opens an agent's shell, types into it, reads the output and replays it",
    () =>
      Effect.gen(function* shell() {
        const { folder, socketPath } = yield* runningDaemon;
        yield* agentIn(folder, socketPath);
        const opened = yield* openTerminal({
          socketPath,
          wire: { cols: 80, id: 1, location: "local", rows: 24 },
        }).pipe(Effect.flatMap(decodeOpenOutcome));
        const terminal = opened.kind === "opened" ? opened.terminal : 0;
        const sent = yield* typed(
          socketPath,
          terminal,
          'echo wake-$((40 + 2))-in-$(basename "$PWD")\n'
        );
        const first = yield* watched({
          from: 0,
          socketPath,
          terminal,
          until: ({ text }) => String.includes("wake-42-in-agent-one")(text),
        });
        const replayed = yield* watched({
          from: 0,
          socketPath,
          terminal,
          until: ({ text }) => String.includes("wake-42-in-agent-one")(text),
        });
        assert.deepStrictEqual(opened, { kind: "opened", terminal: 1 });
        assert.deepStrictEqual(sent, { kind: "sent" });
        assert.include(first.text, "wake-42-in-agent-one");
        assert.strictEqual(first.exited, null);
        assert.include(replayed.text, "wake-42-in-agent-one");
      })
  );

  it.live("resizes the shell and reports its exit code", () =>
    Effect.gen(function* sizeAndExit() {
      const { folder, socketPath } = yield* runningDaemon;
      yield* agentIn(folder, socketPath);
      const opened = yield* openTerminal({
        socketPath,
        wire: { cols: 80, id: 1, location: "local", rows: 24 },
      }).pipe(Effect.flatMap(decodeOpenOutcome));
      const terminal = opened.kind === "opened" ? opened.terminal : 0;
      const resized = yield* resizeTerminal({
        socketPath,
        wire: { cols: 100, rows: 30, terminal },
      }).pipe(Effect.flatMap(decodeSentOutcome));
      yield* typed(
        socketPath,
        terminal,
        "echo size-is-$(stty size | tr ' ' x)\n"
      );
      const sized = yield* watched({
        from: 0,
        socketPath,
        terminal,
        until: ({ text }) => String.includes("size-is-30x100")(text),
      });
      yield* typed(socketPath, terminal, "exit 3\n");
      const ended = yield* watched({
        from: sized.next,
        socketPath,
        terminal,
        until: ({ exited }) => exited !== null,
      });
      const late = yield* typed(socketPath, terminal, "echo too-late\n");
      assert.deepStrictEqual(resized, { kind: "sent" });
      assert.include(sized.text, "size-is-30x100");
      assert.strictEqual(ended.exited, 3);
      assert.deepStrictEqual(late, {
        kind: "refused",
        reason: "the shell in terminal 1 has exited",
      });
    })
  );

  it.live(
    "is refused for a cloud task, an unknown agent and an unknown terminal",
    () =>
      Effect.gen(function* refusals() {
        const { socketPath } = yield* runningDaemon;
        const cloud = yield* openTerminal({
          socketPath,
          wire: { cols: 80, id: 1, location: "cloud", rows: 24 },
        }).pipe(Effect.flatMap(decodeOpenOutcome));
        const unknown = yield* openTerminal({
          socketPath,
          wire: { cols: 80, id: 9, location: "local", rows: 24 },
        }).pipe(Effect.flatMap(decodeOpenOutcome));
        const noTerminal = yield* readTerminal({
          socketPath,
          wire: { after: 0, terminal: 99, wait_ms: 0 },
        }).pipe(Effect.flatMap(decodeReadOutcome));
        assert.deepStrictEqual(cloud, {
          kind: "refused",
          reason:
            "a cloud task has no terminal: its shell would run on another machine",
        });
        assert.deepStrictEqual(unknown, {
          kind: "refused",
          reason: "there is no local agent with ID 9",
        });
        assert.deepStrictEqual(noTerminal, {
          kind: "refused",
          reason: "there is no terminal with ID 99",
        });
      })
  );

  it.live("refuses malformed requests without touching the socket", () =>
    Effect.gen(function* malformed() {
      const socketPath = `${scratch()}/absent.sock`;
      const opened = yield* openTerminal({
        socketPath,
        wire: { cols: "wide", id: 1, location: "local", rows: 24 },
      });
      const written = yield* writeTerminal({
        socketPath,
        wire: { data: 7, terminal: 1 },
      });
      const read = yield* readTerminal({
        socketPath,
        wire: { after: 0, terminal: 1 },
      });
      const resized = yield* resizeTerminal({ socketPath, wire: null });
      assert.strictEqual(opened.kind, "refused");
      assert.strictEqual(written.kind, "refused");
      assert.strictEqual(read.kind, "refused");
      assert.strictEqual(resized.kind, "refused");
    })
  );

  it.live("reports a daemon that is not running", () =>
    Effect.gen(function* down() {
      const socketPath = `${scratch()}/absent.sock`;
      const opened = yield* openTerminal({
        socketPath,
        wire: { cols: 80, id: 1, location: "local", rows: 24 },
      }).pipe(Effect.flatMap(decodeOpenOutcome));
      assert.strictEqual(opened.kind, "daemon_down");
    })
  );
});
