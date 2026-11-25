import * as stylex from "@stylexjs/stylex";
import type { ReactElement } from "react";

import type { LampTone } from "#/renderer/floor/tile-model.ts";
import { color, radius, shadow } from "#/renderer/keel/keel-tokens.stylex.ts";

interface LampProps {
  readonly live: boolean;
  readonly tone: LampTone;
}

const pulse = stylex.keyframes({ "50%": { opacity: 0.3 } });

const tones = stylex.create({
  action: { backgroundColor: color.action },
  danger: { backgroundColor: color.danger },
  idle: { backgroundColor: color.inkFaint },
  success: { backgroundColor: color.success },
  warning: { backgroundColor: color.warning },
});

const toneStyles: Readonly<Record<LampTone, stylex.StyleXStyles>> = {
  action: tones.action,
  danger: tones.danger,
  idle: tones.idle,
  success: tones.success,
  warning: tones.warning,
};

const styles = stylex.create({
  lamp: {
    borderColor: color.edgeStrong,
    borderRadius: radius.pill,
    borderStyle: "solid",
    borderWidth: 1,
    boxShadow: shadow.well,
    boxSizing: "border-box",
    display: "inline-block",
    flexShrink: 0,
    height: 12,
    width: 12,
  },
  live: {
    animationDuration: "1.2s",
    animationIterationCount: "infinite",
    animationName: {
      default: pulse,
      "@media (prefers-reduced-motion: reduce)": "none",
    },
    animationTimingFunction: "ease-in-out",
  },
});

const Lamp = ({ live, tone }: LampProps): ReactElement => (
  <span {...stylex.props(styles.lamp, toneStyles[tone], live && styles.live)} />
);

export { Lamp, toneStyles };
