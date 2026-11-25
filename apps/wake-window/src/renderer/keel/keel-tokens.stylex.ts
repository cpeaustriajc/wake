import * as stylex from "@stylexjs/stylex";

export const color = stylex.defineConsts({
  action: "var(--action)",
  actionEdge: "var(--action-edge)",
  actionHi: "var(--action-hi)",
  actionInk: "var(--action-ink)",
  actionLo: "var(--action-lo)",
  actionPit: "var(--action-pit)",
  actionSoft: "var(--action-soft)",
  actionSoftEdge: "var(--action-soft-edge)",
  bg: "var(--bg)",
  control: "var(--control)",
  controlLo: "var(--control-lo)",
  controlQuiet: "var(--control-quiet)",
  danger: "var(--danger)",
  dangerEdge: "var(--danger-edge)",
  dangerInk: "var(--danger-ink)",
  dangerSoft: "var(--danger-soft)",
  edge: "var(--edge)",
  edgeStrong: "var(--edge-strong)",
  focus: "var(--focus)",
  hairline: "var(--hairline)",
  ink: "var(--ink)",
  inkFaint: "var(--ink-faint)",
  inkMuted: "var(--ink-muted)",
  inverse: "var(--inverse)",
  onAction: "var(--on-action)",
  onInverse: "var(--on-inverse)",
  raised: "var(--raised)",
  scrim: "var(--scrim)",
  success: "var(--success)",
  successEdge: "var(--success-edge)",
  successInk: "var(--success-ink)",
  successSoft: "var(--success-soft)",
  surface: "var(--surface)",
  warning: "var(--warning)",
  warningEdge: "var(--warning-edge)",
  warningInk: "var(--warning-ink)",
  warningSoft: "var(--warning-soft)",
  well: "var(--well)",
});

export const shadow = stylex.defineConsts({
  action: "var(--shadow-action)",
  card: "var(--shadow-card)",
  control: "var(--shadow-control)",
  pop: "var(--shadow-pop)",
  well: "var(--shadow-well)",
});

export const space = stylex.defineConsts({
  s1: "var(--space-1)",
  s2: "var(--space-2)",
  s3: "var(--space-3)",
  s4: "var(--space-4)",
  s5: "var(--space-5)",
  s6: "var(--space-6)",
  s8: "var(--space-8)",
  s12: "var(--space-12)",
});

export const radius = stylex.defineConsts({
  card: "var(--radius-card)",
  chip: "var(--radius-chip)",
  control: "var(--radius-control)",
  key: "var(--radius-key)",
  panel: "var(--radius-panel)",
  pill: "var(--radius-pill)",
});

export const height = stylex.defineConsts({
  lg: "var(--height-lg)",
  md: "var(--height-md)",
  sm: "var(--height-sm)",
  xs: "var(--height-xs)",
});

export const font = stylex.defineConsts({
  mono: "var(--font-mono)",
  sans: "var(--font-sans)",
});

export const opacity = stylex.defineConsts({
  disabled: "var(--opacity-disabled)",
});
