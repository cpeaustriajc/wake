import { BRIDGE_NAME, LIST_AGENTS_CHANNEL } from "#/shared/bridge.ts";
import { contextBridge, ipcRenderer } from "electron";
import type { WakeBridge } from "#/shared/bridge.ts";
import { decodeDaemonAnswer } from "#/shared/daemon-answer.ts";
import { fleetReadingOf } from "#/shared/fleet-reading.ts";

const bridge: WakeBridge = {
  readFleet: async () => {
    const wire: unknown = await ipcRenderer.invoke(LIST_AGENTS_CHANNEL);
    return fleetReadingOf(decodeDaemonAnswer(wire));
  },
};

contextBridge.exposeInMainWorld(BRIDGE_NAME, bridge);
