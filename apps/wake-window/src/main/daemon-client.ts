import { isRecord, textOf } from "#/shared/json.ts";
import type { DaemonAnswer } from "#/shared/daemon-answer.ts";
import type { JsonRecord } from "#/shared/json.ts";
import type { Socket } from "node:net";
import { createConnection } from "node:net";
import { createInterface } from "node:readline";
import { once } from "node:events";

const ANSWER_TIMEOUT_MILLISECONDS = 5000;
const HUNG_UP = "the daemon hung up before it answered";
const JSON_RPC_VERSION = "2.0";
const NO_MESSAGE = "the daemon gave no message";
const REQUEST_ID = 1;

const refused = (reason: string): DaemonAnswer => ({ kind: "refused", reason });

const parsed = (line: string): unknown => {
  try {
    return JSON.parse(line);
  } catch {
    return line;
  }
};

const answerOf = (line: string): DaemonAnswer => {
  const response = parsed(line);
  if (!isRecord(response)) {
    return refused("the daemon's answer is not a JSON object");
  }
  const { error } = response;
  if (isRecord(error)) {
    return refused(textOf(error["message"], NO_MESSAGE));
  }
  if (!("result" in response)) {
    return refused("the daemon's answer has no result");
  }
  return { kind: "answered", result: response["result"] };
};

const reasonOf = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  return "the socket failed";
};

const firstLine = async (
  socket: Readonly<Socket>,
  signal: Readonly<AbortSignal>,
): Promise<string> => {
  const lines = createInterface({ input: socket, signal });
  const first = await lines[Symbol.asyncIterator]().next();
  if (first.done === true) {
    throw new Error(HUNG_UP);
  }
  return first.value;
};

const askDaemon = async (
  socketPath: string,
  method: string,
  parameters: JsonRecord,
): Promise<DaemonAnswer> => {
  const request = { id: REQUEST_ID, jsonrpc: JSON_RPC_VERSION, method, params: parameters };
  const signal = AbortSignal.timeout(ANSWER_TIMEOUT_MILLISECONDS);
  const socket = createConnection(socketPath);
  try {
    await once(socket, "connect", { signal });
    socket.write(`${JSON.stringify(request)}\n`);
    return answerOf(await firstLine(socket, signal));
  } catch (error) {
    return { kind: "unreachable", reason: reasonOf(error), socket: socketPath };
  } finally {
    socket.destroy();
  }
};

export { askDaemon };
