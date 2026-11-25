import { Loading, Show, createMemo, createSignal } from "solid-js";
import { AppHeader } from "#/renderer/app-header.tsx";
import { FloorSkeleton } from "#/renderer/floor/floor-notice.tsx";
import { FloorView } from "#/renderer/floor/floor-view.tsx";
import type { JSX } from "@solidjs/web";
import type { WakeBridge } from "#/shared/bridge.ts";
import { WorkspaceView } from "#/renderer/workspace/workspace-view.tsx";
import { createFloorState } from "#/renderer/floor/floor-state.ts";

interface AppProperties {
  readonly bridge: WakeBridge;
}

const NO_KEY = "";

const LoadingFloor = (): JSX.Element => (
  <div class="wk-floor-frame">
    <div class="wk-floor">
      <FloorSkeleton />
    </div>
  </div>
);

const App = (properties: AppProperties): JSX.Element => {
  const floor = createFloorState(properties.bridge);
  const [openedKey, setOpenedKey] = createSignal(NO_KEY);
  const opened = createMemo(() => floor.tiles().find((tile) => tile.key === openedKey()));
  const open = (key: string): void => {
    setOpenedKey(key);
  };
  return (
    <div class="wk-app">
      <AppHeader reading={floor.reading()} />
      <main class="wk-main">
        <Loading fallback={<LoadingFloor />}>
          <Show
            when={opened()}
            fallback={
              <FloorView
                tiles={floor.tiles()}
                placing={floor.placing}
                reading={floor.reading()}
                onOpen={open}
              />
            }
          >
            {(tile) => (
              <WorkspaceView
                tile={tile()}
                onBack={() => {
                  open(NO_KEY);
                }}
              />
            )}
          </Show>
        </Loading>
      </main>
    </div>
  );
};

export { App };
