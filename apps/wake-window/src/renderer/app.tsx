import * as stylex from "@stylexjs/stylex";
import { Array, Option } from "effect";
import type { ReactElement } from "react";

import { AppHeader } from "#/renderer/app-header.tsx";
import { FloorView } from "#/renderer/floor/floor-view.tsx";
import { space } from "#/renderer/keel/keel-tokens.stylex.ts";
import { tilesIn } from "#/renderer/window/window-context-model.ts";
import { useWindow } from "#/renderer/window/window-provider.tsx";
import { WorkspaceView } from "#/renderer/workspace/workspace-view.tsx";

const styles = stylex.create({
  app: { display: "flex", flexDirection: "column", height: "100vh" },
  main: {
    display: "flex",
    flexDirection: "column",
    flexGrow: 1,
    gap: space.s3,
    minHeight: 0,
    minWidth: 0,
    padding: space.s5,
  },
});

const App = (): ReactElement => {
  const context = useWindow((snapshot) => snapshot.context);
  const inWorkspace = useWindow((snapshot) =>
    snapshot.matches({ navigation: "workspace" })
  );
  const opened = Array.findFirst(
    tilesIn(context),
    (tile) => inWorkspace && tile.key === context.openedKey
  );
  return (
    <div {...stylex.props(styles.app)}>
      <AppHeader />
      <main {...stylex.props(styles.main)}>
        {Option.isSome(opened) ? (
          <WorkspaceView tile={opened.value} />
        ) : (
          <FloorView />
        )}
      </main>
    </div>
  );
};

export { App };
