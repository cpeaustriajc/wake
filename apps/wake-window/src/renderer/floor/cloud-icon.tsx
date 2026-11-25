import * as stylex from "@stylexjs/stylex";
import type { ReactElement } from "react";

import { color } from "#/renderer/keel/keel-tokens.stylex.ts";

const styles = stylex.create({
  icon: { color: color.inkMuted, flexShrink: 0 },
});

const CloudIcon = (): ReactElement => (
  <svg
    {...stylex.props(styles.icon)}
    aria-hidden="true"
    fill="none"
    height="14"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth="1.5"
    viewBox="0 0 16 16"
    width="14"
  >
    <path d="M4.5 12.5h7a3 3 0 0 0 .3-6 4 4 0 0 0-7.6 1A2.5 2.5 0 0 0 4.5 12.5z" />
  </svg>
);

export { CloudIcon };
