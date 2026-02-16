import "./styles.css";
import { D3TermApp } from "./terminal";

const terminalElement = document.querySelector<HTMLElement>("#terminal");
const warningElement = document.querySelector<HTMLElement>("#warning");

if (!terminalElement || !warningElement) {
  throw new Error("terminal root elements are missing");
}

const app = new D3TermApp(terminalElement, warningElement);

void app.init();

window.addEventListener("beforeunload", () => {
  void app.dispose();
});
