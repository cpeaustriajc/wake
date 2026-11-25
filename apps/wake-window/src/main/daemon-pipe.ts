import { decodeFloorPlacement, placementParameters } from "#/shared/floor-placement.ts";
import type { DaemonAnswer } from "#/shared/daemon-answer.ts";
import { askDaemon } from "#/main/daemon-client.ts";

const LIST_AGENTS_METHOD = "agent.list";
const PLACE_ON_FLOOR_METHOD = "floor.place";

const listAgents = async (socketPath: string): Promise<DaemonAnswer> => {
  const answer = await askDaemon(socketPath, LIST_AGENTS_METHOD, {});
  return answer;
};

const placeOnFloor = async (socketPath: string, wire: unknown): Promise<DaemonAnswer> => {
  const placement = decodeFloorPlacement(wire);
  if (!placement.ok) {
    return { kind: "refused", reason: placement.problem };
  }
  const answer = await askDaemon(
    socketPath,
    PLACE_ON_FLOOR_METHOD,
    placementParameters(placement.value),
  );
  return answer;
};

export { listAgents, placeOnFloor };
