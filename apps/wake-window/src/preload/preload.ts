import { BRIDGE_NAME, LIST_AGENTS_CHANNEL, PLACE_ON_FLOOR_CHANNEL } from "#/shared/bridge.ts";
import { contextBridge, ipcRenderer } from "electron";
import type { WakeBridge } from "#/shared/bridge.ts";
import { decodeDaemonAnswer } from "#/shared/daemon-answer.ts";
import { fleetReadingOf } from "#/shared/fleet-reading.ts";
import { placeOutcomeOf } from "#/shared/floor-placement.ts";

const bridge: WakeBridge = {
  placeOnFloor: async (placement) => {
    const wire: unknown = await ipcRenderer.invoke(PLACE_ON_FLOOR_CHANNEL, placement);
    return placeOutcomeOf(decodeDaemonAnswer(wire));
  },
  readFleet: async () => {
    const wire: unknown = await ipcRenderer.invoke(LIST_AGENTS_CHANNEL);
    return fleetReadingOf(decodeDaemonAnswer(wire));
  },
};

contextBridge.exposeInMainWorld(BRIDGE_NAME, bridge);
