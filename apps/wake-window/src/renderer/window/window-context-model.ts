import type { Nudge, PlacementContext, Press, Pull } from "#/renderer/window/placement-context-model.ts";
import type { FleetContext } from "#/renderer/window/fleet-context-model.ts";
import { INITIAL_FLEET } from "#/renderer/window/fleet-context-model.ts";
import { INITIAL_PLACEMENT } from "#/renderer/window/placement-context-model.ts";
import { tilesIn } from "#/renderer/window/placement-context-model.ts";

interface WindowContext extends FleetContext, PlacementContext {
  readonly openedKey: string;
}

type WindowEvent =
  | (Nudge & { readonly type: "tile.nudge" })
  | (Press & { readonly type: "tile.press" })
  | (Pull & { readonly type: "pointer.move" })
  | { readonly key: string; readonly type: "tile.click" | "workspace.open" }
  | { readonly type: "fleet.read" | "pointer.release" | "workspace.close" };

const NO_KEY = "";

const INITIAL_CONTEXT: WindowContext = { ...INITIAL_FLEET, ...INITIAL_PLACEMENT, openedKey: NO_KEY };

const knowsTile = (context: WindowContext, key: string): boolean =>
  tilesIn(context).some((tile) => tile.key === key);

const agentLeft = (context: WindowContext): boolean => !knowsTile(context, context.openedKey);

export { INITIAL_CONTEXT, NO_KEY, agentLeft, knowsTile };
export type { WindowContext, WindowEvent };
