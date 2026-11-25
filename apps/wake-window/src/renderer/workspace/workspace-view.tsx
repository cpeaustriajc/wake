import * as stylex from "@stylexjs/stylex";
import { Tabs, TabsList, TabsPanel, TabsTab } from "@ui-sdk/react/tabs";
import { Array } from "effect";
import type { ReactElement } from "react";

import { Lamp } from "#/renderer/floor/lamp.tsx";
import type { Tile } from "#/renderer/floor/tile-model.ts";
import { color, font, space } from "#/renderer/keel/keel-tokens.stylex.ts";
import { useWindowSend } from "#/renderer/window/window-provider.tsx";

interface Pane {
  readonly label: string;
  readonly later: string;
  readonly missing: string;
  readonly value: string;
}

const THREAD: Pane = {
  label: "Thread",
  later:
    "Messages between you and this agent appear here once the daemon carries them.",
  missing: "No thread yet",
  value: "thread",
};

const PANES: readonly Pane[] = [
  {
    label: "Diff",
    later:
      "Each turn's changes appear here once the daemon reads this agent's branch.",
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
    later:
      "A shell in this agent's folder, kept alive by the daemon, comes next.",
    missing: "No terminal yet",
    value: "terminal",
  },
  {
    label: "Browser",
    later:
      "A page view for the app this agent builds comes after the terminal.",
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

const styles = stylex.create({
  body: {
    alignItems: "flex-start",
    display: "flex",
    flexWrap: "wrap",
    gap: space.s4,
  },
  crumbs: {
    alignItems: "center",
    display: "flex",
    flexWrap: "wrap",
    gap: 6,
  },
  here: {
    fontFamily: font.sans,
    fontSize: 14,
    fontWeight: 600,
    lineHeight: "20px",
  },
  mark: { color: color.inkFaint },
  meta: { color: color.inkMuted, fontSize: 13 },
  panes: {
    display: "flex",
    flexBasis: 420,
    flexDirection: "column",
    flexGrow: 999,
    flexShrink: 1,
    gap: space.s3,
    minWidth: 0,
  },
  phase: { fontWeight: 500 },
  status: {
    alignItems: "center",
    display: "flex",
    flexWrap: "wrap",
    gap: space.s2,
  },
  tabs: { maxWidth: 520 },
  thread: {
    flexBasis: 300,
    flexGrow: 1,
    flexShrink: 1,
    maxWidth: 400,
    minWidth: 0,
  },
  workspace: {
    display: "flex",
    flexDirection: "column",
    flexGrow: 1,
    gap: space.s3,
    minHeight: 0,
    overflow: "auto",
  },
});

const EmptyPane = ({ pane }: { readonly pane: Pane }): ReactElement => (
  <div className="k-card">
    <div className="k-empty">
      <span className="k-empty__title">{pane.missing}</span>
      <span className="k-empty__desc">{pane.later}</span>
    </div>
  </div>
);

const Panes = (): ReactElement => (
  <Tabs {...stylex.props(styles.panes)} defaultValue="diff">
    <div {...stylex.props(styles.tabs)}>
      <TabsList aria-label="Workspace panes" className="k-tabs">
        {Array.map(PANES, ({ label, value }) => (
          <TabsTab className="k-tab" key={value} value={value}>
            {label}
          </TabsTab>
        ))}
      </TabsList>
    </div>
    {Array.map(PANES, (pane) => (
      <TabsPanel key={pane.value} value={pane.value}>
        <EmptyPane pane={pane} />
      </TabsPanel>
    ))}
  </Tabs>
);

const WorkspaceView = ({ tile }: { readonly tile: Tile }): ReactElement => {
  const send = useWindowSend();
  const handleBack = (): void => {
    send({ type: "workspace.close" });
  };
  return (
    <div {...stylex.props(styles.workspace)}>
      <nav {...stylex.props(styles.crumbs)} aria-label="Where you are">
        <button
          className="k-btn k-btn--link k-btn--sm"
          onClick={handleBack}
          type="button"
        >
          Floor
        </button>
        <span {...stylex.props(styles.mark)} aria-hidden="true">
          ›
        </span>
        <span {...stylex.props(styles.here)}>
          {tile.key} · {tile.headline}
        </span>
      </nav>
      <div {...stylex.props(styles.status)}>
        <Lamp live={tile.live} tone={tile.lamp} />
        <span {...stylex.props(styles.phase)}>{tile.status}</span>
        <span {...stylex.props(styles.meta)}>
          {tile.place} · {tile.folder}
        </span>
        <span {...stylex.props(styles.meta)}>{tile.when}</span>
      </div>
      <div {...stylex.props(styles.body)}>
        <section {...stylex.props(styles.thread)} aria-label="Thread">
          <EmptyPane pane={THREAD} />
        </section>
        <Panes />
      </div>
    </div>
  );
};

export { WorkspaceView };
