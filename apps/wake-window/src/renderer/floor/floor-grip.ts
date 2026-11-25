import {
  GRID_STEP,
  draggedTo,
  movedBy,
  positionOf,
  pulledPast,
} from "#/renderer/floor/floor-model.ts";
import type { Grip, Layout, Position } from "#/renderer/floor/floor-model.ts";

interface FloorGripOptions {
  readonly layout: () => Layout;
  readonly onDrop: (key: string, position: Position) => void;
  readonly onMove: (key: string, position: Position) => void;
  readonly onOpen: (key: string) => void;
}

interface FloorGrip {
  readonly drag: (clientX: number, clientY: number) => void;
  readonly grab: (key: string, startX: number, startY: number) => void;
  readonly nudge: (key: string, stepsLeft: number, stepsTop: number) => void;
  readonly open: (key: string) => void;
  readonly release: (key: string) => void;
}

interface Hold {
  readonly grip: Grip;
  readonly moved: boolean;
}

const createFloorGrip = (options: FloorGripOptions): FloorGrip => {
  let holds: readonly Hold[] = [];
  let justMoved = false;
  return {
    drag: (clientX, clientY): void => {
      for (const hold of holds) {
        if (hold.moved || pulledPast(hold.grip, clientX, clientY)) {
          holds = [{ grip: hold.grip, moved: true }];
          options.onMove(hold.grip.key, draggedTo(hold.grip, clientX, clientY));
        }
      }
    },
    grab: (key, startX, startY): void => {
      const origin = positionOf(options.layout(), key);
      holds = [{ grip: { key, origin, startX, startY }, moved: false }];
      justMoved = false;
    },
    nudge: (key, stepsLeft, stepsTop): void => {
      const from = positionOf(options.layout(), key);
      options.onDrop(key, movedBy(from, stepsLeft * GRID_STEP, stepsTop * GRID_STEP));
    },
    open: (key): void => {
      if (justMoved) {
        justMoved = false;
        return;
      }
      options.onOpen(key);
    },
    release: (key): void => {
      justMoved = holds.some((hold) => hold.moved);
      holds = [];
      if (justMoved) {
        options.onDrop(key, positionOf(options.layout(), key));
      }
    },
  };
};

export { createFloorGrip };
