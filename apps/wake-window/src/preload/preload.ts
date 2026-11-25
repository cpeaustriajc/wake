import { contextBridge, ipcRenderer } from "electron";

import {
  BRIDGE_NAME,
  LIST_AGENTS_CHANNEL,
  OPEN_TERMINAL_CHANNEL,
  PLACE_ON_FLOOR_CHANNEL,
  READ_TERMINAL_CHANNEL,
  RESIZE_TERMINAL_CHANNEL,
  WRITE_TERMINAL_CHANNEL,
} from "#/shared/bridge.ts";
import type { WakeBridge } from "#/shared/bridge.ts";

const bridge: WakeBridge = {
  openTerminal: (opening) => ipcRenderer.invoke(OPEN_TERMINAL_CHANNEL, opening),
  placeOnFloor: (placement) =>
    ipcRenderer.invoke(PLACE_ON_FLOOR_CHANNEL, placement),
  readFleet: () => ipcRenderer.invoke(LIST_AGENTS_CHANNEL),
  readTerminal: (reading) => ipcRenderer.invoke(READ_TERMINAL_CHANNEL, reading),
  resizeTerminal: (size) => ipcRenderer.invoke(RESIZE_TERMINAL_CHANNEL, size),
  writeTerminal: (input) => ipcRenderer.invoke(WRITE_TERMINAL_CHANNEL, input),
};

contextBridge.exposeInMainWorld(BRIDGE_NAME, bridge);
