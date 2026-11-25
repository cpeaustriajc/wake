import * as stylex from "@stylexjs/stylex";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { String } from "effect";
import { useEffect, useRef } from "react";
import type { ReactElement } from "react";

import type { TerminalFeed } from "#/renderer/window/terminal-context-model.ts";
import type { WindowEvent } from "#/renderer/window/window-context-model.ts";
import {
  useTerminalFeed,
  useWindowSend,
} from "#/renderer/window/window-provider.tsx";
import {
  FONT_SIZE,
  FONT_TOKEN,
  SCROLLBACK_LINES,
  paletteFrom,
} from "#/renderer/workspace/terminal-palette-model.ts";

interface Mounting {
  readonly element: HTMLElement;
  readonly send: (event: WindowEvent) => void;
  readonly subscribe: (listener: (feed: TerminalFeed) => void) => () => void;
  readonly tileKey: string;
}

const THEME_ATTRIBUTE = "data-theme";

const styles = stylex.create({
  host: { height: "100%", minHeight: 0, minWidth: 0, width: "100%" },
});

const tokenNow = (token: string): string =>
  String.trim(
    getComputedStyle(document.documentElement).getPropertyValue(token)
  );

const unmounted = (): void => undefined;

const mounted = ({
  element,
  send,
  subscribe,
  tileKey,
}: Mounting): (() => void) => {
  const terminal = new Terminal({
    cursorBlink: true,
    fontFamily: tokenNow(FONT_TOKEN),
    fontSize: FONT_SIZE,
    scrollback: SCROLLBACK_LINES,
    theme: paletteFrom(tokenNow),
  });
  const fit = new FitAddon();
  terminal.loadAddon(fit);
  terminal.open(element);
  fit.fit();
  const stopFeed = subscribe((feed) => {
    if (feed.type === "terminal.reset") {
      terminal.reset();
    } else {
      terminal.write(feed.bytes);
    }
  });
  const typing = terminal.onData((data) => {
    send({ data, type: "terminal.type" });
  });
  const resizing = terminal.onResize(({ cols, rows }) => {
    send({ cols, rows, type: "terminal.fit" });
  });
  const sized = new ResizeObserver(() => {
    fit.fit();
  });
  sized.observe(element);
  const themed = new MutationObserver(() => {
    terminal.options.theme = paletteFrom(tokenNow);
  });
  themed.observe(document.documentElement, {
    attributeFilter: [THEME_ATTRIBUTE],
  });
  send({
    cols: terminal.cols,
    key: tileKey,
    rows: terminal.rows,
    type: "terminal.show",
  });
  return () => {
    themed.disconnect();
    sized.disconnect();
    resizing.dispose();
    typing.dispose();
    stopFeed();
    terminal.dispose();
    send({ type: "terminal.hide" });
  };
};

const TerminalHost = ({
  tileKey,
}: {
  readonly tileKey: string;
}): ReactElement => {
  const container = useRef<HTMLDivElement>(null);
  const send = useWindowSend();
  const subscribe = useTerminalFeed();

  useEffect(() => {
    const element = container.current;
    return element === null
      ? unmounted
      : mounted({ element, send, subscribe, tileKey });
  }, [send, subscribe, tileKey]);

  return (
    <div
      {...stylex.props(styles.host)}
      data-terminal={tileKey}
      ref={container}
    />
  );
};

export { TerminalHost };
