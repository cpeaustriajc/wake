import { Effect, Schema } from "effect";

import { askDaemon } from "#/main/daemon-client.ts";
import { FloorPlacement } from "#/shared/agent.ts";
import type { Answer } from "#/shared/daemon-answer.ts";

const LIST_AGENTS_METHOD = "agent.list";
const PLACE_ON_FLOOR_METHOD = "floor.place";

const listAgents = (socketPath: string): Effect.Effect<Answer> =>
  askDaemon({ method: LIST_AGENTS_METHOD, params: {}, socketPath });

interface Request {
  readonly socketPath: string;
  readonly wire: unknown;
}

const placeOnFloor = ({ socketPath, wire }: Request): Effect.Effect<Answer> =>
  Schema.decodeUnknownEffect(FloorPlacement)(wire).pipe(
    Effect.flatMap((placement) =>
      askDaemon({
        method: PLACE_ON_FLOOR_METHOD,
        params: placement,
        socketPath,
      })
    ),
    Effect.catchTag("SchemaError", ({ message }) =>
      Effect.succeed<Answer>({ kind: "refused", reason: message })
    )
  );

export { listAgents, placeOnFloor };
