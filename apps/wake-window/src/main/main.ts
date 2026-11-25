import { Config, Effect } from "effect";
import { BrowserWindow, app, ipcMain } from "electron";

import {
  listAgents,
  openTerminal,
  placeOnFloor,
  readTerminal,
  resizeTerminal,
  writeTerminal,
} from "#/main/daemon-pipe.ts";
import {
  LIST_AGENTS_CHANNEL,
  OPEN_TERMINAL_CHANNEL,
  PLACE_ON_FLOOR_CHANNEL,
  READ_TERMINAL_CHANNEL,
  RESIZE_TERMINAL_CHANNEL,
  WRITE_TERMINAL_CHANNEL,
} from "#/shared/bridge.ts";

const MINIMUM_HEIGHT = 480;
const MINIMUM_WIDTH = 640;
const WINDOW_HEIGHT = 900;
const WINDOW_WIDTH = 1440;

const socketPath = Config.String("WAKE_SOCKET").pipe(
  Config.withDefault(`${app.getPath("home")}/.wake/wake.sock`)
);

const rendererUrl = Config.String("ELECTRON_RENDERER_URL").pipe(
  Config.withDefault(new URL("../renderer/index.html", import.meta.url).href)
);

const openWindow = (home: string): Promise<void> => {
  const viewer = new BrowserWindow({
    height: WINDOW_HEIGHT,
    minHeight: MINIMUM_HEIGHT,
    minWidth: MINIMUM_WIDTH,
    title: "Wake",
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      preload: `${import.meta.dirname}/../preload/preload.cjs`,
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
  return viewer.loadURL(home);
};

const FORWARDED = [
  [PLACE_ON_FLOOR_CHANNEL, placeOnFloor],
  [OPEN_TERMINAL_CHANNEL, openTerminal],
  [READ_TERMINAL_CHANNEL, readTerminal],
  [RESIZE_TERMINAL_CHANNEL, resizeTerminal],
  [WRITE_TERMINAL_CHANNEL, writeTerminal],
] as const;

ipcMain.handle(LIST_AGENTS_CHANNEL, () =>
  Effect.runPromise(socketPath.pipe(Effect.flatMap(listAgents)))
);
for (const [channel, forward] of FORWARDED) {
  ipcMain.handle(channel, (_event, wire: unknown) =>
    Effect.runPromise(
      socketPath.pipe(
        Effect.flatMap((path) => forward({ socketPath: path, wire }))
      )
    )
  );
}
app.on("window-all-closed", () => {
  app.quit();
});
await app.whenReady();
await openWindow(await Effect.runPromise(rendererUrl));
