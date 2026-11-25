import { createConnection } from "node:net";
import type { Socket } from "node:net";

import { Array, Effect, Schema, String } from "effect";

import type { Answer } from "#/shared/daemon-answer.ts";

class DaemonUnreachable extends Schema.TaggedError<
  DaemonUnreachable,
  { readonly brand: unique symbol }
>()("DaemonUnreachable", { reason: Schema.String }) {}

const RpcRequest = Schema.fromJsonString(
  Schema.Struct({
    id: Schema.Int,
    jsonrpc: Schema.Literal("2.0"),
    method: Schema.String,
    params: Schema.Unknown,
  })
);

const RpcResponse = Schema.fromJsonString(
  Schema.Union([
    Schema.Struct({ error: Schema.Struct({ message: Schema.String }) }),
    Schema.Struct({ result: Schema.Unknown }),
  ])
);

const ANSWER_TIMEOUT = "5 seconds";
const HUNG_UP = "the daemon hung up before it answered";
const LATE = "the daemon did not answer in time";
const LINE_END = "\n";
const REQUEST_ID = 1;

const connected = (socketPath: string) =>
  Effect.acquireRelease(
    Effect.callback<Socket, DaemonUnreachable>((resume) => {
      const socket = createConnection(socketPath);
      socket.setEncoding("utf-8");
      socket.once("connect", () => {
        resume(Effect.succeed(socket));
      });
      socket.once("error", (error) => {
        resume(Effect.fail(DaemonUnreachable.make({ reason: error.message })));
      });
    }),
    (socket) =>
      Effect.sync(() => {
        socket.destroy();
      })
  );

const firstLine = (socket: Socket) =>
  Effect.callback<string, DaemonUnreachable>((resume) => {
    const chunks: string[] = [];
    socket.on("data", (chunk: string) => {
      chunks.push(chunk);
      const [line, ...rest] = String.split(Array.join(chunks, ""), LINE_END);
      if (rest.length > 0) {
        resume(Effect.succeed(line));
      }
    });
    socket.once("end", () => {
      resume(Effect.fail(DaemonUnreachable.make({ reason: HUNG_UP })));
    });
    socket.once("error", (error) => {
      resume(Effect.fail(DaemonUnreachable.make({ reason: error.message })));
    });
  });

const exchange = (socketPath: string, request: string) =>
  connected(socketPath).pipe(
    Effect.tap((socket) =>
      Effect.sync(() => {
        socket.write(`${request}${LINE_END}`);
      })
    ),
    Effect.flatMap(firstLine),
    Effect.scoped,
    Effect.timeoutOrElse({
      duration: ANSWER_TIMEOUT,
      orElse: () => Effect.fail(DaemonUnreachable.make({ reason: LATE })),
    })
  );

interface Question {
  readonly method: string;
  readonly params: unknown;
  readonly socketPath: string;
}

const askDaemon = ({
  method,
  params,
  socketPath,
}: Question): Effect.Effect<Answer> =>
  Schema.encodeEffect(RpcRequest)({
    id: REQUEST_ID,
    jsonrpc: "2.0",
    method,
    params,
  }).pipe(
    Effect.flatMap((request) => exchange(socketPath, request)),
    Effect.flatMap(Schema.decodeUnknownEffect(RpcResponse)),
    Effect.map((response): Answer =>
      "error" in response
        ? { kind: "refused", reason: response.error.message }
        : { kind: "answered", result: response.result }
    ),
    Effect.catchTags({
      DaemonUnreachable: ({ reason }) =>
        Effect.succeed<Answer>({
          kind: "unreachable",
          reason,
          socket: socketPath,
        }),
      SchemaError: ({ message }) =>
        Effect.succeed<Answer>({ kind: "refused", reason: message }),
    })
  );

export { askDaemon };
