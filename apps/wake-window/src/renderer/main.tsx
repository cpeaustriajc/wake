import "#/renderer/keel/tokens.css";
import "#/renderer/keel/bundle.css";
import "@xterm/xterm/css/xterm.css";
import { createRoot } from "react-dom/client";

import { App } from "#/renderer/app.tsx";
import { ThemeAttribute } from "#/renderer/theme/theme-attribute.tsx";
import {
  WindowProvider,
  themeOptionsNow,
  windowLogicFor,
} from "#/renderer/window/window-provider.tsx";

const root = document.querySelector("#root");
const logic = windowLogicFor(globalThis.wake);
const themeOptions = themeOptionsNow();

if (root !== null) {
  createRoot(root).render(
    <WindowProvider logic={logic} themeOptions={themeOptions}>
      <ThemeAttribute />
      <App />
    </WindowProvider>
  );
}
