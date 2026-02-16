import { describe, expect, it } from "vitest";

import { DEFAULT_CONFIG, normalizeConfig } from "./config-client";

describe("normalizeConfig", () => {
  it("returns defaults for invalid payload", () => {
    expect(normalizeConfig(null)).toEqual(DEFAULT_CONFIG);
  });

  it("applies valid startup and terminal fields", () => {
    const config = normalizeConfig({
      startup: {
        multiplexer: "tmux",
        shell: "/bin/zsh",
        shell_args: ["-l"],
      },
      terminal: {
        theme: "dark",
        font_family: "Menlo, monospace",
        font_size: 15,
        letter_spacing: -0.5,
        line_height: 1.4,
        scrollback: 4000,
      },
    });

    expect(config.startup.multiplexer).toBe("tmux");
    expect(config.startup.shell).toBe("/bin/zsh");
    expect(config.terminal.theme).toBe("dark");
    expect(config.terminal.font_size).toBe(15);
    expect(config.terminal.letter_spacing).toBe(-0.5);
    expect(config.terminal.scrollback).toBe(4000);
  });

  it("clamps numerical values", () => {
    const config = normalizeConfig({
      terminal: {
        font_size: 2,
        letter_spacing: -99,
        line_height: 99,
        scrollback: -10,
      },
    });

    expect(config.terminal.font_size).toBe(8);
    expect(config.terminal.letter_spacing).toBe(-10);
    expect(config.terminal.line_height).toBe(2.5);
    expect(config.terminal.scrollback).toBe(100);
  });

  it("quotes font family names that include spaces", () => {
    const config = normalizeConfig({
      terminal: {
        font_family: "GoMono Nerd Font Mono, monospace",
      },
    });
    expect(config.terminal.font_family).toBe("'GoMono Nerd Font Mono', monospace");
  });
});
