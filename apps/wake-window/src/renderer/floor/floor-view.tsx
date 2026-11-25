import type { Grip, Layout, Position } from "#/renderer/floor/floor-model.ts";
import {
  GRID_STEP,
  draggedTo,
  extentOf,
  movedBy,
  positionOf,
  pulledPast,
} from "#/renderer/floor/floor-model.ts";
import { For } from "solid-js";
import type { JSX } from "@solidjs/web";
import { MachineTile } from "#/renderer/floor/machine-tile.tsx";
import { Minimap } from "#/renderer/floor/minimap.tsx";
import type { Tile } from "#/renderer/floor/tile-model.ts";

interface FloorViewProperties {
  readonly children: JSX.Element;
  readonly layout: Layout;
  readonly onMove: (key: string, position: Position) => void;
  readonly onOpen: (key: string) => void;
  readonly tiles: readonly Tile[];
}

interface Hold {
  readonly grip: Grip;
  readonly moved: boolean;
}

const FloorView = (properties: FloorViewProperties): JSX.Element => {
  let holds: readonly Hold[] = [];
  let justMoved = false;
  const extent = (): Position =>
    extentOf(properties.tiles.map((tile) => positionOf(properties.layout, tile.key)));
  const grab = (key: string, startX: number, startY: number): void => {
    const origin = positionOf(properties.layout, key);
    holds = [{ grip: { key, origin, startX, startY }, moved: false }];
    justMoved = false;
  };
  const drag = (clientX: number, clientY: number): void => {
    for (const hold of holds) {
      if (hold.moved || pulledPast(hold.grip, clientX, clientY)) {
        holds = [{ grip: hold.grip, moved: true }];
        properties.onMove(hold.grip.key, draggedTo(hold.grip, clientX, clientY));
      }
    }
  };
  const release = (): void => {
    justMoved = holds.some((hold) => hold.moved);
    holds = [];
  };
  const open = (key: string): void => {
    if (justMoved) {
      justMoved = false;
      return;
    }
    properties.onOpen(key);
  };
  const nudge = (key: string, stepsX: number, stepsY: number): void => {
    const from = positionOf(properties.layout, key);
    properties.onMove(key, movedBy(from, stepsX * GRID_STEP, stepsY * GRID_STEP));
  };
  return (
    <div class="wk-floor-frame">
      <div class="wk-floor">
        <span
          class="wk-floor__extent"
          style={{ height: `${extent().y}px`, width: `${extent().x}px` }}
          aria-hidden="true"
        />
        <For each={properties.tiles} keyed={(tile) => tile.key}>
          {(tile) => (
            <MachineTile
              tile={tile()}
              position={positionOf(properties.layout, tile().key)}
              onGrab={grab}
              onDrag={drag}
              onRelease={release}
              onOpen={open}
              onNudge={nudge}
            />
          )}
        </For>
        {properties.children}
      </div>
      <Minimap tiles={properties.tiles} layout={properties.layout} />
    </div>
  );
};

export { FloorView };
