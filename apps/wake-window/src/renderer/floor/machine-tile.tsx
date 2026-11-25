import { CloudIcon } from "#/renderer/floor/cloud-icon.tsx";
import type { JSX } from "@solidjs/web";
import { Lamp } from "#/renderer/floor/lamp.tsx";
import type { Position } from "#/renderer/floor/floor-model.ts";
import { Show } from "solid-js";
import type { Tile } from "#/renderer/floor/tile-model.ts";

interface MachineTileProperties {
  readonly onDrag: (clientX: number, clientY: number) => void;
  readonly onGrab: (key: string, clientX: number, clientY: number) => void;
  readonly onNudge: (key: string, stepsX: number, stepsY: number) => void;
  readonly onOpen: (key: string) => void;
  readonly onRelease: () => void;
  readonly position: Position;
  readonly tile: Tile;
}

const NUDGES: Readonly<Record<string, readonly [number, number]>> = {
  ArrowDown: [0, 1],
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
};

const MachineTile = (properties: MachineTileProperties): JSX.Element => (
  <button
    type="button"
    class="wk-tile"
    style={{ left: `${properties.position.x}px`, top: `${properties.position.y}px` }}
    aria-label={`${properties.tile.key}, ${properties.tile.headline}, ${properties.tile.status}, ${properties.tile.place}`}
    aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown Alt+ArrowLeft Alt+ArrowRight"
    onClick={() => {
      properties.onOpen(properties.tile.key);
    }}
    onPointerDown={(event) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      properties.onGrab(properties.tile.key, event.clientX, event.clientY);
    }}
    onPointerMove={(event) => {
      properties.onDrag(event.clientX, event.clientY);
    }}
    onPointerUp={() => {
      properties.onRelease();
    }}
    onPointerCancel={() => {
      properties.onRelease();
    }}
    onKeyDown={(event) => {
      const nudge = NUDGES[event.key];
      if (event.altKey && nudge) {
        event.preventDefault();
        properties.onNudge(properties.tile.key, ...nudge);
      }
    }}
  >
    <span class="wk-tile__head">
      <span class="wk-tile__who">
        <Lamp tone={properties.tile.lamp} live={properties.tile.live} />
        <span class="wk-id">{properties.tile.key}</span>
        <Show when={properties.tile.cloud}>
          <CloudIcon />
        </Show>
      </span>
      <span class="wk-status">{properties.tile.status}</span>
    </span>
    <span class="wk-headline">{properties.tile.headline}</span>
    <span class="wk-note">{properties.tile.note}</span>
    <span class="wk-tile__foot">
      <span class="wk-when">{properties.tile.when}</span>
    </span>
  </button>
);

export { MachineTile };
