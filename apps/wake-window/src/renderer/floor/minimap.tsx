import * as stylex from "@stylexjs/stylex";
import { Array } from "effect";
import type { ReactElement } from "react";

import { markOf, positionOf } from "#/renderer/floor/floor-model.ts";
import type { Layout } from "#/renderer/floor/floor-model.ts";
import { toneStyles } from "#/renderer/floor/lamp.tsx";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import {
  color,
  radius,
  shadow,
  space,
} from "#/renderer/keel/keel-tokens.stylex.ts";

interface MinimapProps {
  readonly layout: Layout;
  readonly tiles: readonly Tile[];
}

const styles = stylex.create({
  box: (left: number, top: number, width: number, height: number) => ({
    height,
    left,
    top,
    width,
  }),
  map: {
    backgroundColor: color.surface,
    borderColor: color.edge,
    borderRadius: radius.panel,
    borderStyle: "solid",
    borderWidth: 1,
    bottom: space.s3,
    boxShadow: shadow.pop,
    paddingBlock: 6,
    paddingInline: space.s2,
    pointerEvents: "none",
    position: "absolute",
    right: space.s3,
    zIndex: 3,
  },
  mark: {
    borderColor: color.edgeStrong,
    borderRadius: radius.key,
    borderStyle: "solid",
    borderWidth: 1,
    boxSizing: "border-box",
    position: "absolute",
  },
  well: {
    backgroundColor: color.well,
    borderColor: color.edgeStrong,
    borderRadius: radius.chip,
    borderStyle: "solid",
    borderWidth: 1,
    boxShadow: shadow.well,
    height: 136,
    overflow: "hidden",
    position: "relative",
    width: 156,
  },
});

const Minimap = ({ layout, tiles }: MinimapProps): ReactElement => (
  <div {...stylex.props(styles.map)} aria-hidden="true">
    <div {...stylex.props(styles.well)}>
      {Array.map(tiles, ({ key, lamp }) => {
        const mark = markOf(positionOf({ key, layout }));
        return (
          <span
            {...stylex.props(
              styles.mark,
              toneStyles[lamp],
              styles.box(mark.left, mark.top, mark.width, mark.height)
            )}
            data-mark={key}
            key={key}
          />
        );
      })}
    </div>
  </div>
);

export { Minimap };
