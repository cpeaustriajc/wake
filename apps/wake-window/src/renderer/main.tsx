import "#/renderer/keel/tokens.css";
import "#/renderer/keel/bundle.css";
import { createRoot } from "react-dom/client";

import { App } from "#/renderer/app.tsx";
import { ThemeAttribute } from "#/renderer/theme/theme-attribute.tsx";
import { WindowProvider } from "#/renderer/window/window-provider.tsx";

const root = document.querySelector("#root");

if (root !== null) {
  createRoot(root).render(
    <WindowProvider bridge={globalThis.wake}>
      <ThemeAttribute />
      <App />
    </WindowProvider>
  );
}
