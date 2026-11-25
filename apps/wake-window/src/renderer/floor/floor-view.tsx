import { FloorNotice, PlaceAlert } from "#/renderer/floor/floor-notice.tsx";
import { extentOf, positionOf } from "#/renderer/floor/floor-model.ts";
import type { FleetReading } from "#/shared/fleet-reading.ts";
import { For } from "solid-js";
import type { JSX } from "@solidjs/web";
import { MachineTile } from "#/renderer/floor/machine-tile.tsx";
import { Minimap } from "#/renderer/floor/minimap.tsx";
import type { Placing } from "#/renderer/floor/floor-placing.ts";
import type { Position } from "#/renderer/floor/floor-model.ts";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import { createFloorGrip } from "#/renderer/floor/floor-grip.ts";

interface FloorViewProperties {
  readonly onOpen: (key: string) => void;
  readonly placing: Placing;
  readonly reading: FleetReading;
  readonly tiles: readonly Tile[];
}

const FloorView = (properties: FloorViewProperties): JSX.Element => {
  const { drop, failures, layout, move } = properties.placing;
  const grip = createFloorGrip({
    layout,
    onDrop: drop,
    onMove: move,
    onOpen: (key) => {
      properties.onOpen(key);
    },
  });
  const extent = (): Position =>
    extentOf(properties.tiles.map((tile) => positionOf(layout(), tile.key)));
  return (
    <div class="wk-floor-frame">
      <div class="wk-floor">
        <span
          class="wk-floor__extent"
          style={{ height: `${extent().top}px`, width: `${extent().left}px` }}
          aria-hidden="true"
        />
        <For
          each={properties.tiles}
          keyed={(tile) => tile.key}
          fallback={<FloorNotice reading={properties.reading} />}
        >
          {(tile) => (
            <MachineTile
              tile={tile()}
              position={positionOf(layout(), tile().key)}
              onGrab={grip.grab}
              onDrag={grip.drag}
              onRelease={grip.release}
              onOpen={grip.open}
              onNudge={grip.nudge}
            />
          )}
        </For>
      </div>
      <PlaceAlert failures={failures()} />
      <Minimap tiles={properties.tiles} layout={layout()} />
    </div>
  );
};

export { FloorView };
