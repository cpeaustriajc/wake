import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";

import { assert, describe, it } from "@effect/vitest";
import { Effect, Schedule } from "effect";

import { askDaemon } from "#/main/daemon-client.ts";
import { listAgents, placeOnFloor } from "#/main/daemon-pipe.ts";
import {
  decodeFleetReading,
  decodePlaceOutcome,
} from "#/shared/daemon-answer.ts";

const BINARY = new URL("../../../target/debug/wake", import.meta.url).pathname;
const PROGRESS =
  "---\nphase: build\nheadline: Adding the parser\nupdated: 2026-11-24T14:00:00Z\n---\nNext: tests\n";

const scratch = (): string => mkdtempSync(`${tmpdir()}/wake-window-`);

const runningDaemon = Effect.acquireRelease(
  Effect.sync(() => {
    const folder = scratch();
    const socketPath = `${folder}/d.sock`;
    const daemon = spawn(BINARY, ["daemon"], {
      env: { HOME: folder, WAKE_DB: `${folder}/d.db`, WAKE_SOCKET: socketPath },
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

const workspace = (folder: string): string => {
  const path = `${folder}/agent`;
  mkdirSync(`${path}/.wake`, { recursive: true });
  writeFileSync(`${path}/.wake/progress.md`, PROGRESS);
  return path;
};

describe("the main process against the real daemon", () => {
  it("needs the wake binary, built with cargo build -p wake-cli", () => {
    assert.isTrue(
      existsSync(BINARY),
      `${BINARY} is missing: run cargo build -p wake-cli from the repository root`
    );
  });

  it.live("lists a registered agent, places it and reads the place back", () =>
    Effect.gen(function* contract() {
      const { folder, socketPath } = yield* runningDaemon;
      const path = workspace(folder);
      const registered = yield* askDaemon({
        method: "agent.register",
        params: { folder: path },
        socketPath,
      });
      const before = yield* Effect.flatMap(
        listAgents(socketPath),
        decodeFleetReading
      );
      const placed = yield* placeOnFloor({
        socketPath,
        wire: { id: 1, location: "local", x: 360, y: 40 },
      }).pipe(Effect.flatMap(decodePlaceOutcome));
      const after = yield* Effect.flatMap(
        listAgents(socketPath),
        decodeFleetReading
      );
      assert.deepStrictEqual(registered, {
        kind: "answered",
        result: { id: 1 },
      });
      assert.deepStrictEqual(before, {
        agents: [
          {
            branch: null,
            folder: path,
            id: 1,
            location: "local",
            position: null,
            progress: {
              headline: "Adding the parser",
              phase: "build",
              state: "found",
              updated: "2026-11-24T14:00:00Z",
            },
            started_at:
              before.kind === "agents"
                ? (before.agents[0]?.started_at ?? 0)
                : 0,
          },
        ],
        kind: "agents",
      });
      assert.deepStrictEqual(placed, { kind: "placed" });
      assert.deepStrictEqual(
        after.kind === "agents" ? after.agents[0]?.position : "unreadable",
        { x: 360, y: 40 }
      );
    })
  );

  it.live(
    "is refused for an agent the daemon does not know and for a negative place",
    () =>
      Effect.gen(function* refusals() {
        const { socketPath } = yield* runningDaemon;
        const unknown = yield* placeOnFloor({
          socketPath,
          wire: { id: 7, location: "local", x: 40, y: 40 },
        }).pipe(Effect.flatMap(decodePlaceOutcome));
        const negative = yield* placeOnFloor({
          socketPath,
          wire: { id: 7, location: "cloud", x: -20, y: 40 },
        }).pipe(Effect.flatMap(decodePlaceOutcome));
        assert.deepStrictEqual(unknown, {
          kind: "refused",
          reason: "there is no local agent with ID 7",
        });
        assert.deepStrictEqual(negative, {
          kind: "refused",
          reason: "a floor position cannot be negative, and (-20, 40) is",
        });
      })
  );

  it.live("refuses a malformed placement without touching the socket", () =>
    Effect.gen(function* malformed() {
      const answer = yield* placeOnFloor({
        socketPath: `${scratch()}/absent.sock`,
        wire: { id: "one", location: "local", x: 40, y: 40 },
      });
      assert.strictEqual(answer.kind, "refused");
    })
  );

  it.live("reports a daemon that is not running", () =>
    Effect.gen(function* down() {
      const socketPath = `${scratch()}/absent.sock`;
      const reading = yield* Effect.flatMap(
        listAgents(socketPath),
        decodeFleetReading
      );
      assert.strictEqual(reading.kind, "daemon_down");
      assert.strictEqual(
        reading.kind === "daemon_down" ? reading.socket : "",
        socketPath
      );
    })
  );
});
