import type { JSX } from "@solidjs/web";
import type { LampTone } from "#/renderer/floor/tile-model.ts";

interface LampProperties {
  readonly live: boolean;
  readonly tone: LampTone;
}

const Lamp = (properties: LampProperties): JSX.Element => (
  <span class={["wk-lamp", `wk-lamp--${properties.tone}`, { "wk-pulse": properties.live }]} />
);

export { Lamp };
