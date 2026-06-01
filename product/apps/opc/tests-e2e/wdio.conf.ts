// Spec 187 FR-T1/FR-T5 — WebdriverIO config for the built-binary e2e run.
//
// This drives a *built* OPC binary through tauri-driver, which has a
// Linux/Windows WebView backend only (no macOS WKWebView WebDriver). It is
// therefore NOT run locally on macOS and NOT part of ci-gate; it runs in the
// nightly (.github/workflows/opc-e2e-nightly.yml) on ubuntu-latest, where the
// nightly installs the WebdriverIO toolchain + `cargo install tauri-driver` +
// the WebKitWebDriver system package.
//
// Pattern follows the Tauri v2 WebDriver guide:
//   https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/
//
// Excluded from the local tsconfig (it imports the WebdriverIO toolchain the
// nightly provides); the nightly's runner loads it via tsx.

import { spawn, type ChildProcess } from "node:child_process";
import { opcBinaryPath } from "./harness/driver";

let tauriDriver: ChildProcess | undefined;

export const config = {
  runner: "local",

  // tauri-driver listens here and proxies to the native WebView driver.
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",

  specs: ["./fixtures/**/*.e2e.ts"],
  // Serial: a single OPC binary instance per session keeps process-tree and
  // boot-gate assertions deterministic (FR-T3/FR-T6 — no flake amnesty).
  maxInstances: 1,

  capabilities: [
    {
      // tauri-driver selects the Tauri WebView via browserName "wry" and reads
      // the binary to launch from tauri:options.application.
      browserName: "wry",
      "tauri:options": {
        application: opcBinaryPath(),
      },
    },
  ],

  framework: "mocha",
  mochaOpts: {
    ui: "bdd",
    // The BootGate poll cadence is ~1s; AC-7/AC-8 windows need headroom.
    timeout: 120_000,
  },
  reporters: ["spec"],

  // tauri-driver must be on PATH (`cargo install tauri-driver`) and the runner
  // must have WebKitWebDriver installed (`apt-get install -y webkit2gtk-driver`
  // on Linux). Start it before the session and kill it after — tauri-driver in
  // turn spawns and reaps the platform webdriver.
  onPrepare: () => {
    tauriDriver = spawn("tauri-driver", [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },
  onComplete: () => {
    tauriDriver?.kill();
  },
};
