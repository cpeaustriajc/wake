import { Effect, Schema } from "effect";

import { ANSWER_MILLISECONDS, askDaemon } from "#/main/daemon-client.ts";
import { FloorPlacement } from "#/shared/agent.ts";
import type { Answer } from "#/shared/daemon-answer.ts";
import {
  TerminalInput,
  TerminalOpening,
  TerminalReading,
  TerminalSize,
} from "#/shared/terminal.ts";

const LIST_AGENTS_METHOD = "agent.list";
const PLACE_ON_FLOOR_METHOD = "floor.place";
const OPEN_TERMINAL_METHOD = "terminal.open";
const READ_TERMINAL_METHOD = "terminal.read";
const RESIZE_TERMINAL_METHOD = "terminal.resize";
const WRITE_TERMINAL_METHOD = "terminal.write";
const LONGEST_WAIT_MILLISECONDS = 30_000;

const listAgents = (socketPath: string): Effect.Effect<Answer> =>
  askDaemon({ method: LIST_AGENTS_METHOD, params: {}, socketPath });

interface Request {
  readonly socketPath: string;
  readonly wire: unknown;
}

const refusedFor = ({ message }: Schema.SchemaError): Effect.Effect<Answer> =>
  Effect.succeed({ kind: "refused", reason: message });

const placeOnFloor = ({ socketPath, wire }: Request): Effect.Effect<Answer> =>
  Schema.decodeUnknownEffect(FloorPlacement)(wire).pipe(
    Effect.flatMap((placement) =>
      askDaemon({
        method: PLACE_ON_FLOOR_METHOD,
        params: placement,
        socketPath,
      })
    ),
    Effect.catchTag("SchemaError", refusedFor)
  );

const openTerminal = ({ socketPath, wire }: Request): Effect.Effect<Answer> =>
  Schema.decodeUnknownEffect(TerminalOpening)(wire).pipe(
    Effect.flatMap((opening) =>
      askDaemon({ method: OPEN_TERMINAL_METHOD, params: opening, socketPath })
    ),
    Effect.catchTag("SchemaError", refusedFor)
  );

const writeTerminal = ({ socketPath, wire }: Request): Effect.Effect<Answer> =>
  Schema.decodeUnknownEffect(TerminalInput)(wire).pipe(
    Effect.flatMap((input) =>
      askDaemon({ method: WRITE_TERMINAL_METHOD, params: input, socketPath })
    ),
    Effect.catchTag("SchemaError", refusedFor)
  );

const resizeTerminal = ({ socketPath, wire }: Request): Effect.Effect<Answer> =>
  Schema.decodeUnknownEffect(TerminalSize)(wire).pipe(
    Effect.flatMap((size) =>
      askDaemon({ method: RESIZE_TERMINAL_METHOD, params: size, socketPath })
    ),
    Effect.catchTag("SchemaError", refusedFor)
  );

const readTerminal = ({ socketPath, wire }: Request): Effect.Effect<Answer> =>
  Schema.decodeUnknownEffect(TerminalReading)(wire).pipe(
    Effect.flatMap((reading) =>
      askDaemon({
        method: READ_TERMINAL_METHOD,
        params: reading,
        patienceMillis:
          Math.min(Math.max(reading.wait_ms, 0), LONGEST_WAIT_MILLISECONDS) +
          ANSWER_MILLISECONDS,
        socketPath,
      })
    ),
    Effect.catchTag("SchemaError", refusedFor)
  );

export {
  listAgents,
  openTerminal,
  placeOnFloor,
  readTerminal,
  resizeTerminal,
  writeTerminal,
};
