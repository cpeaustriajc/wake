import * as stylex from "@stylexjs/stylex";
import { Array } from "effect";
import type { ReactElement } from "react";

import {
  GRID_STEP,
  extentOf,
  positionOf,
} from "#/renderer/floor/floor-model.ts";
import {
  FloorAlerts,
  FloorNotice,
  FloorSkeleton,
} from "#/renderer/floor/floor-notice.tsx";
import { MachineTile } from "#/renderer/floor/machine-tile.tsx";
import { Minimap } from "#/renderer/floor/minimap.tsx";
import {
  color,
  radius,
  shadow,
  space,
} from "#/renderer/keel/keel-tokens.stylex.ts";
import { layoutIn, tilesIn } from "#/renderer/window/window-context-model.ts";
import {
  useWindow,
  useWindowSend,
} from "#/renderer/window/window-provider.tsx";

const styles = stylex.create({
  extent: (width: number, height: number) => ({
    height,
    left: 0,
    pointerEvents: "none",
    position: "absolute",
    top: 0,
    width,
  }),
  floor: {
    backgroundAttachment: "local",
    backgroundColor: color.well,
    backgroundImage: `linear-gradient(${color.hairline} 1px, transparent 1px), linear-gradient(90deg, ${color.hairline} 1px, transparent 1px)`,
    borderColor: color.edgeStrong,
    borderRadius: radius.card,
    borderStyle: "solid",
    borderWidth: 1,
    boxShadow: shadow.well,
    inset: 0,
    overflow: "auto",
    position: "absolute",
  },
  column: {
    display: "flex",
    flexDirection: "column",
    flexGrow: 1,
    gap: space.s3,
    minHeight: 0,
  },
  frame: { flexGrow: 1, minHeight: 0, position: "relative" },
  grid: (step: number) => ({ backgroundSize: `${step}px ${step}px` }),
});

const FloorView = (): ReactElement => {
  const send = useWindowSend();
  const context = useWindow((snapshot) => snapshot.context);
  const connecting = useWindow((snapshot) =>
    snapshot.matches({ fleet: { status: "connecting" } })
  );
  const tiles = tilesIn(context);
  const layout = layoutIn(context);
  const extent = extentOf(
    Array.map(tiles, ({ key }) => positionOf({ key, layout }))
  );
  const empty = !Array.isReadonlyArrayNonEmpty(tiles);
  return (
    <div {...stylex.props(styles.column)}>
      <FloorAlerts
        failures={context.failures}
        stale={!empty}
        trouble={context.trouble}
      />
      <div {...stylex.props(styles.frame)}>
        <div {...stylex.props(styles.floor, styles.grid(GRID_STEP))} data-floor>
          <span
            {...stylex.props(styles.extent(extent.x, extent.y))}
            aria-hidden="true"
          />
          {Array.map(tiles, (tile) => (
            <MachineTile
              key={tile.key}
              onEvent={send}
              position={positionOf({ key: tile.key, layout })}
              tile={tile}
            />
          ))}
          {connecting ? <FloorSkeleton /> : null}
          {empty && !connecting ? (
            <FloorNotice trouble={context.trouble} />
          ) : null}
        </div>
        <Minimap layout={layout} tiles={tiles} />
      </div>
    </div>
  );
};

export { FloorView };
