import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal } from "@xterm/xterm";

import {
  DEFAULT_CONFIG,
  normalizeConfig,
  type AppConfig,
  type ConfigUpdatedPayload,
} from "./config-client";
import "@xterm/xterm/css/xterm.css";

interface PtyDataPayload {
  data: string;
}

interface SessionExitPayload {
  code: number | null;
}

interface WarningPayload {
  message: string;
}

interface SessionInfo {
  pid: number | null;
  command: string;
  fallback_used: boolean;
}

const DARK_THEME = {
  background: "#0b1020",
  foreground: "#d4d9e5",
  cursor: "#f7f9ff",
  selectionBackground: "#2a35547a",
  selectionInactiveBackground: "#212c4672",
  black: "#0f1422",
  red: "#e47884",
  green: "#7dc9a8",
  yellow: "#d9bf7a",
  blue: "#62bbea",
  magenta: "#d777e6",
  cyan: "#6ec9db",
  white: "#dbe2ef",
  brightBlack: "#667086",
  brightRed: "#f08e9a",
  brightGreen: "#99dbbe",
  brightYellow: "#ead29b",
  brightBlue: "#8bccef",
  brightMagenta: "#e19aeb",
  brightCyan: "#95d9e6",
  brightWhite: "#f1f4fb",
};

const LIGHT_THEME = {
  background: "#f8fafc",
  foreground: "#273245",
  cursor: "#1b2330",
  selectionBackground: "#c6d8eb8f",
  selectionInactiveBackground: "#d8e4f393",
  black: "#212936",
  red: "#bf5f75",
  green: "#2f7f5f",
  yellow: "#9b7934",
  blue: "#326fa1",
  magenta: "#8f55c4",
  cyan: "#337d99",
  white: "#ecf0f8",
  brightBlack: "#4e5869",
  brightRed: "#cf6f85",
  brightGreen: "#3c9974",
  brightYellow: "#b69449",
  brightBlue: "#4889be",
  brightMagenta: "#9f6bd1",
  brightCyan: "#4c95b0",
  brightWhite: "#ffffff",
};

export class D3TermApp {
  private terminal: Terminal;

  private fitAddon: FitAddon;

  private unlisteners: UnlistenFn[] = [];

  private config: AppConfig = DEFAULT_CONFIG;

  private prefersDark = window.matchMedia("(prefers-color-scheme: dark)");

  private resizeObserver: ResizeObserver | null = null;

  private resizeRafId: number | null = null;

  private warningTimerId: number | null = null;

  private inputBuffer = "";

  private inputTimerId: number | null = null;

  private backendAvailable = isTauri();

  constructor(
    private readonly terminalContainer: HTMLElement,
    private readonly warningContainer: HTMLElement,
  ) {
    this.fitAddon = new FitAddon();
    this.terminal = new Terminal({
      cursorBlink: true,
      allowProposedApi: true,
      convertEol: false,
      drawBoldTextInBrightColors: true,
      fontWeight: 500,
      fontWeightBold: 700,
      minimumContrastRatio: 1.1,
      letterSpacing: DEFAULT_CONFIG.terminal.letter_spacing,
      rescaleOverlappingGlyphs: false,
      rightClickSelectsWord: true,
      fontFamily: DEFAULT_CONFIG.terminal.font_family,
      fontSize: DEFAULT_CONFIG.terminal.font_size,
      lineHeight: DEFAULT_CONFIG.terminal.line_height,
      scrollback: DEFAULT_CONFIG.terminal.scrollback,
    });
  }

  async init(): Promise<void> {
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.loadAddon(new WebLinksAddon());
    this.terminal.loadAddon(new Unicode11Addon());
    this.terminal.unicode.activeVersion = "11";

    this.terminal.open(this.terminalContainer);
    this.applyConfig(DEFAULT_CONFIG);

    this.registerResizeHandling();
    this.prefersDark.addEventListener("change", this.handleThemeChange);
    this.fitAndResize();

    if (!this.backendAvailable) {
      this.terminal.writeln("d3term backend is unavailable in browser preview mode.");
      this.terminal.writeln("Run `npm run tauri dev` and use the Tauri window.");
      this.showWarning("ブラウザ直アクセスではPTYを起動できません。Tauriウィンドウで実行してください。");
      return;
    }

    await this.registerBackendEvents();
    this.registerInputHandler();
    await this.startSession();
  }

  async dispose(): Promise<void> {
    for (const unlisten of this.unlisteners) {
      unlisten();
    }
    this.unlisteners = [];

    this.prefersDark.removeEventListener("change", this.handleThemeChange);
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    window.removeEventListener("resize", this.scheduleResize);

    if (this.resizeRafId !== null) {
      window.cancelAnimationFrame(this.resizeRafId);
      this.resizeRafId = null;
    }

    if (this.inputTimerId !== null) {
      window.clearTimeout(this.inputTimerId);
      this.inputTimerId = null;
    }

    if (this.warningTimerId !== null) {
      window.clearTimeout(this.warningTimerId);
      this.warningTimerId = null;
    }

    if (this.backendAvailable) {
      await invoke("stop_session").catch(() => undefined);
    }
    this.terminal.dispose();
  }

  private async registerBackendEvents(): Promise<void> {
    this.unlisteners.push(
      await listen<PtyDataPayload>("pty:data", (event) => {
        this.terminal.write(event.payload.data);
      }),
    );

    this.unlisteners.push(
      await listen<SessionExitPayload>("session:exit", (event) => {
        const codeText = event.payload.code === null ? "unknown" : String(event.payload.code);
        this.terminal.writeln(`\r\n[process exited: ${codeText}]`);
      }),
    );

    this.unlisteners.push(
      await listen<WarningPayload>("warning", (event) => {
        this.showWarning(event.payload.message);
      }),
    );

    this.unlisteners.push(
      await listen<ConfigUpdatedPayload>("config:updated", (event) => {
        const next = normalizeConfig(event.payload.config);
        this.applyConfig(next);
        this.fitAndResize();
      }),
    );
  }

  private registerInputHandler(): void {
    if (!this.backendAvailable) {
      return;
    }
    this.terminal.onData((data) => {
      this.inputBuffer += data;
      if (this.inputTimerId !== null) {
        return;
      }
      this.inputTimerId = window.setTimeout(() => {
        const chunk = this.inputBuffer;
        this.inputBuffer = "";
        this.inputTimerId = null;
        void invoke("write_stdin", { data: chunk }).catch(() => undefined);
      }, 4);
    });
  }

  private registerResizeHandling(): void {
    this.resizeObserver = new ResizeObserver(() => {
      this.scheduleResize();
    });
    this.resizeObserver.observe(this.terminalContainer);
    window.addEventListener("resize", this.scheduleResize);
  }

  private readonly scheduleResize = (): void => {
    if (this.resizeRafId !== null) {
      return;
    }
    this.resizeRafId = window.requestAnimationFrame(() => {
      this.resizeRafId = null;
      this.fitAndResize();
    });
  };

  private fitAndResize(): void {
    this.fitAddon.fit();
    const cols = Math.max(2, this.terminal.cols);
    const rows = Math.max(1, this.terminal.rows);
    void invoke("resize", { cols, rows }).catch(() => undefined);
  }

  private async startSession(): Promise<void> {
    const cols = Math.max(2, this.terminal.cols);
    const rows = Math.max(1, this.terminal.rows);
    const info = await invoke<SessionInfo>("start_session", { cols, rows });
    if (info.fallback_used) {
      this.showWarning("指定コマンドを使えないため、通常シェルで起動しました。");
    }
    this.terminal.focus();
  }

  private applyConfig(next: AppConfig): void {
    this.config = next;
    this.terminal.options.fontFamily = next.terminal.font_family;
    this.terminal.options.fontSize = next.terminal.font_size;
    this.terminal.options.letterSpacing = next.terminal.letter_spacing;
    this.terminal.options.lineHeight = next.terminal.line_height;
    this.terminal.options.scrollback = next.terminal.scrollback;
    this.applyTheme();
  }

  private readonly handleThemeChange = (): void => {
    if (this.config.terminal.theme === "system") {
      this.applyTheme();
    }
  };

  private applyTheme(): void {
    const mode = this.config.terminal.theme;
    const dark = mode === "dark" || (mode === "system" && this.prefersDark.matches);
    this.terminal.options.theme = dark ? DARK_THEME : LIGHT_THEME;
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  }

  private showWarning(message: string): void {
    this.warningContainer.textContent = message;
    this.warningContainer.classList.add("is-visible");
    if (this.warningTimerId !== null) {
      window.clearTimeout(this.warningTimerId);
    }
    this.warningTimerId = window.setTimeout(() => {
      this.warningContainer.classList.remove("is-visible");
    }, 4500);
  }
}
