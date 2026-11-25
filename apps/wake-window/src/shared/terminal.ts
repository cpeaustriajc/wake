import { Effect, Schema } from "effect";

import { Location } from "#/shared/agent.ts";
import { DaemonAnswer } from "#/shared/daemon-answer.ts";

const TerminalOpening = Schema.Struct({
  cols: Schema.Int,
  id: Schema.Int,
  location: Location,
  rows: Schema.Int,
});

const TerminalInput = Schema.Struct({
  data: Schema.String,
  terminal: Schema.Int,
});

const TerminalSize = Schema.Struct({
  cols: Schema.Int,
  rows: Schema.Int,
  terminal: Schema.Int,
});

const TerminalReading = Schema.Struct({
  after: Schema.Int,
  terminal: Schema.Int,
  wait_ms: Schema.Int,
});

const OpenedTerminal = Schema.Struct({ terminal: Schema.Int });

const TerminalOutput = Schema.Struct({
  data: Schema.Uint8ArrayFromBase64,
  exited: Schema.Int.pipe(Schema.NullOr, Schema.optionalKey),
  next: Schema.Int,
  truncated: Schema.Boolean,
});

const Sent = Schema.Struct({});

type Opening = typeof TerminalOpening.Type;
type Input = typeof TerminalInput.Type;
type Resize = typeof TerminalSize.Type;
type Reading = typeof TerminalReading.Type;

interface TerminalTrouble {
  readonly kind: "daemon_down" | "refused";
  readonly reason: string;
}

type OpenOutcome =
  | TerminalTrouble
  | { readonly kind: "opened"; readonly terminal: number };

type SentOutcome = TerminalTrouble | { readonly kind: "sent" };

type ReadOutcome =
  | TerminalTrouble
  | {
      readonly bytes: Uint8Array;
      readonly exited: number | null;
      readonly kind: "output";
      readonly next: number;
      readonly truncated: boolean;
    };

const NOT_AN_ANSWER =
  "the window's own bridge sent something that is not an answer";

const resultIn = (wire: unknown): Effect.Effect<unknown, TerminalTrouble> =>
  Schema.decodeUnknownEffect(DaemonAnswer)(wire).pipe(
    Effect.mapError((): TerminalTrouble => ({
      kind: "refused",
      reason: NOT_AN_ANSWER,
    })),
    Effect.flatMap((answer) => {
      if (answer.kind === "answered") {
        return Effect.succeed(answer.result);
      }
      return Effect.fail<TerminalTrouble>({
        kind: answer.kind === "unreachable" ? "daemon_down" : "refused",
        reason: answer.reason,
      });
    })
  );

const unreadable = ({ message }: Schema.SchemaError): TerminalTrouble => ({
  kind: "refused",
  reason: message,
});

const decodeOpenOutcome = (wire: unknown): Effect.Effect<OpenOutcome> =>
  resultIn(wire).pipe(
    Effect.flatMap((result) =>
      Schema.decodeUnknownEffect(OpenedTerminal)(result).pipe(
        Effect.mapError(unreadable)
      )
    ),
    Effect.match({
      onFailure: (trouble): OpenOutcome => trouble,
      onSuccess: ({ terminal }): OpenOutcome => ({ kind: "opened", terminal }),
    })
  );

const decodeSentOutcome = (wire: unknown): Effect.Effect<SentOutcome> =>
  resultIn(wire).pipe(
    Effect.flatMap((result) =>
      Schema.decodeUnknownEffect(Sent)(result).pipe(Effect.mapError(unreadable))
    ),
    Effect.match({
      onFailure: (trouble): SentOutcome => trouble,
      onSuccess: (): SentOutcome => ({ kind: "sent" }),
    })
  );

const decodeReadOutcome = (wire: unknown): Effect.Effect<ReadOutcome> =>
  resultIn(wire).pipe(
    Effect.flatMap((result) =>
      Schema.decodeUnknownEffect(TerminalOutput)(result).pipe(
        Effect.mapError(unreadable)
      )
    ),
    Effect.match({
      onFailure: (trouble): ReadOutcome => trouble,
      onSuccess: ({ data, exited, next, truncated }): ReadOutcome => ({
        bytes: data,
        exited: exited ?? null,
        kind: "output",
        next,
        truncated,
      }),
    })
  );

export {
  TerminalInput,
  TerminalOpening,
  TerminalReading,
  TerminalSize,
  decodeOpenOutcome,
  decodeReadOutcome,
  decodeSentOutcome,
};
export type {
  Input,
  OpenOutcome,
  Opening,
  ReadOutcome,
  Reading,
  Resize,
  SentOutcome,
  TerminalTrouble,
};
