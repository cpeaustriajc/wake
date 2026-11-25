import { Tabs, TabsList, TabsPanel, TabsTab } from "@ui-sdk/solid/tabs";
import { For } from "solid-js";
import type { JSX } from "@solidjs/web";
import { Lamp } from "#/renderer/floor/lamp.tsx";
import type { Tile } from "#/renderer/floor/tile-model.ts";

interface WorkspaceViewProperties {
  readonly onBack: () => void;
  readonly tile: Tile;
}

interface Pane {
  readonly label: string;
  readonly later: string;
  readonly missing: string;
  readonly value: string;
}

const THREAD: Pane = {
  label: "Thread",
  later: "Messages between you and this agent appear here once the daemon carries them.",
  missing: "No thread yet",
  value: "thread",
};

const PANES: readonly Pane[] = [
  {
    label: "Diff",
    later: "Each turn's changes appear here once the daemon reads this agent's branch.",
    missing: "No diff yet",
    value: "diff",
  },
  {
    label: "Files",
    later: "A read-only tree of this agent's folder comes after the diff.",
    missing: "No files yet",
    value: "files",
  },
  {
    label: "Terminal",
    later: "A shell in this agent's folder, kept alive by the daemon, comes next.",
    missing: "No terminal yet",
    value: "terminal",
  },
  {
    label: "Browser",
    later: "A page view for the app this agent builds comes after the terminal.",
    missing: "No browser yet",
    value: "browser",
  },
  {
    label: "Simulator",
    later: "The iOS Simulator's screen needs a Mac with Xcode and comes last.",
    missing: "No simulator yet",
    value: "simulator",
  },
];

const EmptyPane = (properties: { readonly pane: Pane }): JSX.Element => (
  <div class="k-empty">
    <span class="k-empty__title">{properties.pane.missing}</span>
    <span class="k-empty__desc">{properties.pane.later}</span>
  </div>
);

const WorkspaceView = (properties: WorkspaceViewProperties): JSX.Element => (
  <div class="wk-workspace">
    <nav class="wk-crumbs" aria-label="Where you are">
      <button
        type="button"
        class="k-btn k-btn--link k-btn--sm"
        onClick={() => {
          properties.onBack();
        }}
      >
        Floor
      </button>
      <span class="wk-crumbs__mark" aria-hidden="true">
        ›
      </span>
      <span class="wk-crumbs__here">
        {properties.tile.key} · {properties.tile.headline}
      </span>
    </nav>
    <div class="wk-workspace__status">
      <Lamp tone={properties.tile.lamp} live={properties.tile.live} />
      <span class="wk-workspace__phase">{properties.tile.status}</span>
      <span class="wk-workspace__meta">
        {properties.tile.place} · {properties.tile.folder}
      </span>
      <span class="wk-workspace__meta">{properties.tile.when}</span>
    </div>
    <div class="wk-workspace__body">
      <section class="k-card wk-thread" aria-label="Thread">
        <EmptyPane pane={THREAD} />
      </section>
      <Tabs class="wk-panes" defaultValue="diff">
        <TabsList class="k-tabs wk-panes__tabs" aria-label="Workspace panes">
          <For each={PANES}>
            {(pane) => (
              <TabsTab class="k-tab" value={pane.value}>
                {pane.label}
              </TabsTab>
            )}
          </For>
        </TabsList>
        <For each={PANES}>
          {(pane) => (
            <TabsPanel class="k-card" value={pane.value}>
              <EmptyPane pane={pane} />
            </TabsPanel>
          )}
        </For>
      </Tabs>
    </div>
  </div>
);

export { WorkspaceView };
