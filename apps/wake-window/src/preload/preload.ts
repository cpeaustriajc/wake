import { contextBridge, ipcRenderer } from "electron";

import {
  BRIDGE_NAME,
  LIST_AGENTS_CHANNEL,
  PLACE_ON_FLOOR_CHANNEL,
} from "#/shared/bridge.ts";
import type { WakeBridge } from "#/shared/bridge.ts";

const bridge: WakeBridge = {
  placeOnFloor: (placement) =>
    ipcRenderer.invoke(PLACE_ON_FLOOR_CHANNEL, placement),
  readFleet: () => ipcRenderer.invoke(LIST_AGENTS_CHANNEL),
};

contextBridge.exposeInMainWorld(BRIDGE_NAME, bridge);
