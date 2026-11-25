import { isRecord, textOf } from "#/shared/json.ts";

type DaemonAnswer =
  | { readonly kind: "answered"; readonly result: unknown }
  | { readonly kind: "refused"; readonly reason: string }
  | { readonly kind: "unreachable"; readonly reason: string; readonly socket: string };

const UNREADABLE = "the answer could not be read";

const decodeDaemonAnswer = (wire: unknown): DaemonAnswer => {
  if (!isRecord(wire)) {
    return { kind: "refused", reason: UNREADABLE };
  }
  const { kind, reason, result, socket } = wire;
  if (kind === "answered") {
    return { kind, result };
  }
  if (kind === "unreachable") {
    return { kind, reason: textOf(reason, UNREADABLE), socket: textOf(socket, "") };
  }
  return { kind: "refused", reason: textOf(reason, UNREADABLE) };
};

export { decodeDaemonAnswer };
export type { DaemonAnswer };
