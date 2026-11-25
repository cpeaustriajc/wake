import { failure, success } from "#/shared/outcome.ts";
import type { DaemonAnswer } from "#/shared/daemon-answer.ts";
import type { JsonRecord } from "#/shared/json.ts";
import type { Outcome } from "#/shared/outcome.ts";
import { isRecord } from "#/shared/json.ts";

interface FloorPlacement {
  readonly id: number;
  readonly left: number;
  readonly location: "cloud" | "local";
  readonly top: number;
}

type PlaceOutcome =
  | { readonly kind: "daemon_down" | "refused"; readonly reason: string }
  | { readonly kind: "placed" };

const decodeFloorPlacement = (wire: unknown): Outcome<FloorPlacement> => {
  if (!isRecord(wire)) {
    return failure("a placement is not an object");
  }
  const { id, left, location, top } = wire;
  if (location !== "cloud" && location !== "local") {
    return failure("a placement names neither a local agent nor a cloud task");
  }
  if (![id, left, top].every((value) => Number.isSafeInteger(value))) {
    return failure("a placement's id, left and top are not whole numbers");
  }
  if (typeof id !== "number" || typeof left !== "number" || typeof top !== "number") {
    return failure("a placement's id, left and top are not numbers");
  }
  return success({ id, left, location, top });
};

const placementParameters = (placement: FloorPlacement): JsonRecord => ({
  id: placement.id,
  location: placement.location,
  x: placement.left,
  y: placement.top,
});

const placeOutcomeOf = (answer: DaemonAnswer): PlaceOutcome => {
  if (answer.kind === "unreachable") {
    return { kind: "daemon_down", reason: answer.reason };
  }
  if (answer.kind === "refused") {
    return { kind: "refused", reason: answer.reason };
  }
  return { kind: "placed" };
};

export { decodeFloorPlacement, placeOutcomeOf, placementParameters };
export type { FloorPlacement, PlaceOutcome };
