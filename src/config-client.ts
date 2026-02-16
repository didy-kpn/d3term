export type MultiplexerMode = "none" | "tmux" | "zellij";
export type ThemeMode = "system" | "dark" | "light";

export interface StartupConfig {
  multiplexer: MultiplexerMode;
  shell: string | null;
  shell_args: string[];
  zellij_command: string;
  tmux_command: string;
}

export interface TerminalConfig {
  theme: ThemeMode;
  font_family: string;
  font_size: number;
  letter_spacing: number;
  line_height: number;
  scrollback: number;
}

export interface AppConfig {
  startup: StartupConfig;
  terminal: TerminalConfig;
}

export interface ConfigUpdatedPayload {
  config: AppConfig;
  path: string;
}

const DEFAULT_STARTUP: StartupConfig = {
  multiplexer: "zellij",
  shell: null,
  shell_args: [],
  zellij_command: "zellij attach -c d3term",
  tmux_command: "tmux new-session -A -s main",
};

const DEFAULT_TERMINAL: TerminalConfig = {
  theme: "system",
  font_family: "'JetBrains Mono', Menlo, monospace",
  font_size: 13,
  letter_spacing: 0,
  line_height: 1.2,
  scrollback: 10000,
};

export const DEFAULT_CONFIG: AppConfig = {
  startup: DEFAULT_STARTUP,
  terminal: DEFAULT_TERMINAL,
};

function asRecord(value: unknown): Record<string, unknown> | null {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

function asString(value: unknown, fallback: string): string {
  return typeof value === "string" && value.trim().length > 0 ? value : fallback;
}

function normalizeFontFamily(value: unknown, fallback: string): string {
  const raw = asString(value, fallback).trim();
  if (raw.length === 0) {
    return fallback;
  }

  const families = raw
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);

  if (families.length === 0) {
    return fallback;
  }

  const generic = new Set([
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-monospace",
    "emoji",
    "math",
    "fangsong",
  ]);

  const normalized = families.map((family) => {
    if (family.startsWith("'") || family.startsWith('"')) {
      return family;
    }
    if (generic.has(family.toLowerCase())) {
      return family;
    }
    if (/\s/.test(family)) {
      return `'${family.replace(/'/g, "\\'")}'`;
    }
    return family;
  });

  return normalized.join(", ");
}

function asNumber(value: unknown, fallback: number, min: number, max: number): number {
  if (typeof value !== "number" || Number.isNaN(value)) {
    return fallback;
  }
  return Math.max(min, Math.min(max, value));
}

function asStringArray(value: unknown, fallback: string[]): string[] {
  if (!Array.isArray(value)) {
    return fallback;
  }
  const items = value.filter((entry): entry is string => typeof entry === "string");
  return items.length === value.length ? items : fallback;
}

function asMultiplexer(value: unknown): MultiplexerMode {
  if (value === "none" || value === "tmux" || value === "zellij") {
    return value;
  }
  return DEFAULT_STARTUP.multiplexer;
}

function asTheme(value: unknown): ThemeMode {
  if (value === "system" || value === "dark" || value === "light") {
    return value;
  }
  return DEFAULT_TERMINAL.theme;
}

export function normalizeConfig(candidate: unknown): AppConfig {
  const root = asRecord(candidate);
  if (!root) {
    return DEFAULT_CONFIG;
  }

  const startup = asRecord(root.startup);
  const terminal = asRecord(root.terminal);

  return {
    startup: {
      multiplexer: asMultiplexer(startup?.multiplexer),
      shell: typeof startup?.shell === "string" ? startup.shell : null,
      shell_args: asStringArray(startup?.shell_args, DEFAULT_STARTUP.shell_args),
      zellij_command: asString(startup?.zellij_command, DEFAULT_STARTUP.zellij_command),
      tmux_command: asString(startup?.tmux_command, DEFAULT_STARTUP.tmux_command),
    },
    terminal: {
      theme: asTheme(terminal?.theme),
      font_family: normalizeFontFamily(terminal?.font_family, DEFAULT_TERMINAL.font_family),
      font_size: asNumber(terminal?.font_size, DEFAULT_TERMINAL.font_size, 8, 72),
      letter_spacing: asNumber(terminal?.letter_spacing, DEFAULT_TERMINAL.letter_spacing, -10, 10),
      line_height: asNumber(terminal?.line_height, DEFAULT_TERMINAL.line_height, 1, 2.5),
      scrollback: Math.round(
        asNumber(terminal?.scrollback, DEFAULT_TERMINAL.scrollback, 100, 200000),
      ),
    },
  };
}
