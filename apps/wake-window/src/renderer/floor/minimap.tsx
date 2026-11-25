import type { LampTone, Tile } from "#/renderer/floor/tile-model.ts";
import type { Layout, Mark } from "#/renderer/floor/floor-model.ts";
import { markOf, positionOf } from "#/renderer/floor/floor-model.ts";
import { For } from "solid-js";
import type { JSX } from "@solidjs/web";

interface MinimapProperties {
  readonly layout: Layout;
  readonly tiles: readonly Tile[];
}

interface Spot {
  readonly key: string;
  readonly mark: Mark;
  readonly tone: LampTone;
}

const Minimap = (properties: MinimapProperties): JSX.Element => {
  const spots = (): readonly Spot[] =>
    properties.tiles.map((tile) => ({
      key: tile.key,
      mark: markOf(positionOf(properties.layout, tile.key)),
      tone: tile.lamp,
    }));
  return (
    <div class="wk-map" aria-hidden="true">
      <div class="k-well wk-map__well">
        <For each={spots()} keyed={(spot) => spot.key}>
          {(spot) => (
            <span
              class={["wk-map__mark", `wk-lamp--${spot().tone}`]}
              style={{
                height: `${spot().mark.height}px`,
                left: `${spot().mark.left}px`,
                top: `${spot().mark.top}px`,
                width: `${spot().mark.width}px`,
              }}
            />
          )}
        </For>
      </div>
    </div>
  );
};

export { Minimap };
