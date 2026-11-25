import "#/renderer/keel/tokens.css";
import "#/renderer/keel/bundle.css";
import "#/renderer/wake.css";
import { App } from "#/renderer/app.tsx";
import { render } from "@solidjs/web";

const root = document.querySelector("#root");

if (root instanceof HTMLElement) {
  render(() => <App bridge={globalThis.wake} />, root);
}
