import { FloorNotice, FloorSkeleton } from "#/renderer/floor/floor-notice.tsx";
import { Loading, Show, createEffect, createMemo, createSignal, onSettled, refresh } from "solid-js";
import { THEME_LABELS, otherTheme } from "#/renderer/theme/theme-model.ts";
import { decodedLayout, encodedLayout, placedAll, withPosition } from "#/renderer/floor/floor-model.ts";
import type { FleetReading } from "#/shared/fleet-reading.ts";
import { FloorView } from "#/renderer/floor/floor-view.tsx";
import type { JSX } from "@solidjs/web";
import type { Position } from "#/renderer/floor/floor-model.ts";
import type { WakeBridge } from "#/shared/bridge.ts";
import { WorkspaceView } from "#/renderer/workspace/workspace-view.tsx";
import { createTheme } from "#/renderer/theme/theme.ts";
import { tilesOf } from "#/renderer/floor/tile-model.ts";

interface AppProperties {
  readonly bridge: WakeBridge;
}

const LAYOUT_KEY = "wake-floor-layout";
const NOT_APPLICABLE = "—";
const NO_KEY = "";
const POLL_MILLISECONDS = 2000;

const countOf = (reading: FleetReading): string => {
  if (reading.kind !== "agents") {
    return NOT_APPLICABLE;
  }
  if (reading.agents.length === 1) {
    return "1 agent";
  }
  return `${reading.agents.length} agents`;
};

const App = (properties: AppProperties): JSX.Element => {
  const { theme, toggle } = createTheme();
  const reading = createMemo(async () => properties.bridge.readFleet());
  const [now, setNow] = createSignal(Date.now());
  const [openedKey, setOpenedKey] = createSignal(NO_KEY);
  const [saved, setSaved] = createSignal(decodedLayout(localStorage.getItem(LAYOUT_KEY)));
  const tiles = createMemo(() => {
    const fleet = reading();
    if (fleet.kind !== "agents") {
      return [];
    }
    return tilesOf(fleet.agents, now());
  });
  const layout = createMemo(() =>
    placedAll(
      saved(),
      tiles().map((tile) => tile.key),
    ),
  );
  const opened = createMemo(() => tiles().find((tile) => tile.key === openedKey()));
  const move = (key: string, position: Position): void => {
    setSaved(withPosition(layout(), key, position));
  };
  createEffect(layout, (placed) => {
    localStorage.setItem(LAYOUT_KEY, encodedLayout(placed));
    setSaved(placed);
  });
  onSettled(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
      refresh(reading);
    }, POLL_MILLISECONDS);
    return () => {
      clearInterval(timer);
    };
  });
  return (
    <div class="wk-app">
      <header class="wk-header">
        <span class="wk-title">Wake</span>
        <Loading>
          <span class="k-badge k-badge--quiet">{countOf(reading())}</span>
        </Loading>
        <span class="wk-spacer" />
        <button
          type="button"
          class="k-btn k-btn--secondary k-btn--sm"
          aria-label={`Switch to the ${otherTheme(theme())} theme`}
          onClick={toggle}
        >
          {THEME_LABELS[otherTheme(theme())]}
        </button>
      </header>
      <main class="wk-main">
        <Loading
          fallback={
            <div class="wk-floor-frame">
              <div class="wk-floor">
                <FloorSkeleton />
              </div>
            </div>
          }
        >
          <Show
            when={opened()}
            fallback={
              <FloorView tiles={tiles()} layout={layout()} onMove={move} onOpen={setOpenedKey}>
                <Show when={tiles().length === 0}>
                  <FloorNotice reading={reading()} />
                </Show>
              </FloorView>
            }
          >
            {(tile) => (
              <WorkspaceView
                tile={tile()}
                onBack={() => {
                  setOpenedKey(NO_KEY);
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
