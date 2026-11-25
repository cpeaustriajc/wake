import { Record } from "effect";

const PALETTE_TOKENS = {
  background: "--well",
  black: "--ink-faint",
  blue: "--action-ink",
  brightBlack: "--ink-muted",
  brightBlue: "--action",
  brightCyan: "--success",
  brightGreen: "--success",
  brightMagenta: "--action",
  brightRed: "--danger",
  brightWhite: "--ink",
  brightYellow: "--warning",
  cursor: "--action",
  cursorAccent: "--well",
  cyan: "--success-ink",
  foreground: "--ink",
  green: "--success-ink",
  magenta: "--action-ink",
  red: "--danger-ink",
  selectionBackground: "--action-soft",
  white: "--ink-muted",
  yellow: "--warning-ink",
} as const;

const FONT_TOKEN = "--font-mono";
const FONT_SIZE = 13;
const SCROLLBACK_LINES = 5000;

type TerminalPalette = Record<keyof typeof PALETTE_TOKENS, string>;

const paletteFrom = (read: (token: string) => string): TerminalPalette =>
  Record.map(PALETTE_TOKENS, (token) => read(token));

export { FONT_SIZE, FONT_TOKEN, PALETTE_TOKENS, SCROLLBACK_LINES, paletteFrom };
export type { TerminalPalette };
