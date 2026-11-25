import { BrowserWindow, app, ipcMain } from "electron";
import { LIST_AGENTS_CHANNEL, PLACE_ON_FLOOR_CHANNEL } from "#/shared/bridge.ts";
import { listAgents, placeOnFloor } from "#/main/daemon-pipe.ts";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { socketPathOf } from "#/main/socket-path.ts";

const MINIMUM_HEIGHT = 480;
const MINIMUM_WIDTH = 640;
const PRELOAD_FILE = "../preload/preload.cjs";
const RENDERER_FILE = "../renderer/index.html";
const WINDOW_HEIGHT = 900;
const WINDOW_WIDTH = 1440;

const socketPath = socketPathOf(process.env["WAKE_SOCKET"]);

const rendererUrl = (): string => {
  const served = process.env["ELECTRON_RENDERER_URL"];
  if (typeof served === "string") {
    return served;
  }
  return pathToFileURL(path.join(import.meta.dirname, RENDERER_FILE)).href;
};

const openWindow = async (): Promise<void> => {
  const home = rendererUrl();
  const viewer = new BrowserWindow({
    height: WINDOW_HEIGHT,
    minHeight: MINIMUM_HEIGHT,
    minWidth: MINIMUM_WIDTH,
    title: "Wake",
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      preload: path.join(import.meta.dirname, PRELOAD_FILE),
      sandbox: true,
    },
    width: WINDOW_WIDTH,
  });
  viewer.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  viewer.webContents.on("will-navigate", (event, url) => {
    if (url !== home) {
      event.preventDefault();
    }
  });
  await viewer.loadURL(home);
};

ipcMain.handle(LIST_AGENTS_CHANNEL, async () => {
  const answer = await listAgents(socketPath);
  return answer;
});
ipcMain.handle(PLACE_ON_FLOOR_CHANNEL, async (_event, wire: unknown) => {
  const answer = await placeOnFloor(socketPath, wire);
  return answer;
});
app.on("window-all-closed", () => {
  app.quit();
});
await app.whenReady();
await openWindow();
