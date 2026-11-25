import * as stylex from "@stylexjs/stylex";
import type { KeyboardEvent, PointerEvent, ReactElement } from "react";

import { CloudIcon } from "#/renderer/floor/cloud-icon.tsx";
import { MACHINE_HEIGHT, MACHINE_WIDTH } from "#/renderer/floor/floor-model.ts";
import { Lamp } from "#/renderer/floor/lamp.tsx";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import {
  color,
  font,
  radius,
  shadow,
  space,
} from "#/renderer/keel/keel-tokens.stylex.ts";
import type { WindowEvent } from "#/renderer/window/window-context-model.ts";
import type { Position } from "#/shared/agent.ts";

interface MachineTileProps {
  readonly onEvent: (event: WindowEvent) => void;
  readonly position: Position;
  readonly tile: Tile;
}

const NUDGES: Readonly<Record<string, readonly [number, number]>> = {
  ArrowDown: [0, 1],
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
};

const styles = stylex.create({
  foot: {
    gap: 6,
    alignItems: "center",
    color: color.inkMuted,
    display: "flex",
    fontFamily: font.mono,
    fontSize: 11,
    fontWeight: 500,
    lineHeight: "16px",
    marginTop: "auto",
  },
  head: {
    gap: space.s2,
    alignItems: "center",
    display: "flex",
    justifyContent: "space-between",
  },
  headline: {
    overflow: "hidden",
    fontFamily: font.sans,
    fontSize: 14,
    fontWeight: 600,
    lineHeight: "20px",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  id: {
    fontFamily: font.mono,
    fontSize: 12,
    fontWeight: 500,
    lineHeight: "16px",
  },
  note: {
    overflow: "hidden",
    color: color.inkMuted,
    fontFamily: font.mono,
    fontSize: 12,
    fontWeight: 400,
    lineHeight: "16px",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  place: (x: number, y: number, width: number, height: number) => ({
    height,
    left: x,
    top: y,
    width,
  }),
  status: {
    color: color.inkMuted,
    fontSize: 12,
    lineHeight: "16px",
    whiteSpace: "nowrap",
  },
  tile: {
    margin: 0,
    padding: space.s3,
    borderColor: color.edgeStrong,
    borderRadius: radius.panel,
    borderStyle: "solid",
    borderWidth: 1,
    gap: space.s2,
    overflow: "hidden",
    appearance: "none",
    backgroundImage: `linear-gradient(180deg, ${color.control}, ${color.controlLo})`,
    boxShadow: shadow.control,
    boxSizing: "border-box",
    color: color.ink,
    cursor: "pointer",
    display: "flex",
    filter: { default: null, ":hover": "brightness(1.04)" },
    flexDirection: "column",
    position: "absolute",
    textAlign: "left",
    touchAction: "none",
    userSelect: "none",
  },
  who: { gap: 6, alignItems: "center", display: "flex", minWidth: 0 },
});

const MachineTile = ({
  onEvent,
  position,
  tile,
}: MachineTileProps): ReactElement => {
  const { key } = tile;
  const handlePointerDown = (event: PointerEvent<HTMLButtonElement>): void => {
    event.currentTarget.setPointerCapture(event.pointerId);
    onEvent({
      clientX: event.clientX,
      clientY: event.clientY,
      key,
      type: "tile.press",
    });
  };
  const handlePointerMove = (event: PointerEvent<HTMLButtonElement>): void => {
    onEvent({
      clientX: event.clientX,
      clientY: event.clientY,
      type: "pointer.move",
    });
  };
  const handleRelease = (): void => {
    onEvent({ type: "pointer.release" });
  };
  const handleClick = (): void => {
    onEvent({ key, type: "tile.click" });
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>): void => {
    const nudge = NUDGES[event.key];
    if (event.altKey && nudge !== undefined) {
      event.preventDefault();
      onEvent({ key, stepsX: nudge[0], stepsY: nudge[1], type: "tile.nudge" });
    }
  };
  return (
    <button
      {...stylex.props(
        styles.tile,
        styles.place(position.x, position.y, MACHINE_WIDTH, MACHINE_HEIGHT)
      )}
      aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown Alt+ArrowLeft Alt+ArrowRight"
      aria-label={`${key}, ${tile.headline}, ${tile.status}, ${tile.place}`}
      data-machine={key}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      onPointerCancel={handleRelease}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handleRelease}
      type="button"
    >
      <span {...stylex.props(styles.head)}>
        <span {...stylex.props(styles.who)}>
          <Lamp live={tile.live} tone={tile.lamp} />
          <span {...stylex.props(styles.id)}>{key}</span>
          {tile.location === "cloud" ? <CloudIcon /> : null}
        </span>
        <span {...stylex.props(styles.status)}>{tile.status}</span>
      </span>
      <span {...stylex.props(styles.headline)}>{tile.headline}</span>
      <span {...stylex.props(styles.note)}>{tile.note}</span>
      <span {...stylex.props(styles.foot)}>{tile.when}</span>
    </button>
  );
};

export { MachineTile };
