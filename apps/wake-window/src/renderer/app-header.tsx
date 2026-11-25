import * as stylex from "@stylexjs/stylex";
import type { ReactElement } from "react";

import { color, font, space } from "#/renderer/keel/keel-tokens.stylex.ts";
import { THEME_LABELS, otherTheme } from "#/renderer/theme/theme-model.ts";
import {
  useTheme,
  useThemeSend,
  useWindow,
} from "#/renderer/window/window-provider.tsx";

const NOT_APPLICABLE = "—";
const plurals = new Intl.PluralRules("en");

const styles = stylex.create({
  header: {
    gap: space.s3,
    paddingBlock: 10,
    paddingInline: space.s5,
    alignItems: "center",
    backgroundColor: color.surface,
    display: "flex",
    flexWrap: "wrap",
    borderBottomColor: color.hairline,
    borderBottomStyle: "solid",
    borderBottomWidth: 1,
  },
  spacer: { flexGrow: 1 },
  title: {
    fontFamily: font.sans,
    fontSize: 16,
    fontWeight: 600,
    lineHeight: "22px",
  },
});

const countLabel = (count: number): string =>
  `${count} ${plurals.select(count) === "one" ? "agent" : "agents"}`;

const AppHeader = (): ReactElement => {
  const theme = useTheme();
  const sendTheme = useThemeSend();
  const count = useWindow((snapshot) => snapshot.context.agents.length);
  const connecting = useWindow((snapshot) =>
    snapshot.matches({ fleet: { status: "connecting" } })
  );
  const other = otherTheme(theme);
  const handleToggle = (): void => {
    sendTheme({ type: "theme.toggled" });
  };
  return (
    <header {...stylex.props(styles.header)}>
      <span {...stylex.props(styles.title)}>Wake</span>
      <span className="k-badge k-badge--quiet">
        {connecting ? NOT_APPLICABLE : countLabel(count)}
      </span>
      <span {...stylex.props(styles.spacer)} />
      <button
        aria-label={`Switch to the ${other} theme`}
        className="k-btn k-btn--secondary k-btn--sm"
        onClick={handleToggle}
        type="button"
      >
        {THEME_LABELS[other]}
      </button>
    </header>
  );
};

export { AppHeader };
