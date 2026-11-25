import { THEME_LABELS, otherTheme } from "#/renderer/theme/theme-model.ts";
import type { FleetReading } from "#/shared/fleet-reading.ts";
import type { JSX } from "@solidjs/web";
import { Loading } from "solid-js";
import { createTheme } from "#/renderer/theme/theme.ts";

interface AppHeaderProperties {
  readonly reading: FleetReading;
}

const AGENT_NOUNS: Readonly<Partial<Record<Intl.LDMLPluralRule, string>>> = { one: "agent" };
const NOT_APPLICABLE = "—";
const plurals = new Intl.PluralRules("en");

const countOf = (reading: FleetReading): string => {
  if (reading.kind !== "agents") {
    return NOT_APPLICABLE;
  }
  const count = reading.agents.length;
  return `${count} ${AGENT_NOUNS[plurals.select(count)] ?? "agents"}`;
};

const AppHeader = (properties: AppHeaderProperties): JSX.Element => {
  const { theme, toggle } = createTheme();
  return (
    <header class="wk-header">
      <span class="wk-title">Wake</span>
      <Loading>
        <span class="k-badge k-badge--quiet">{countOf(properties.reading)}</span>
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
  );
};

export { AppHeader, countOf };
