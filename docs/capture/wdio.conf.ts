import path from "node:path";
import { fileURLToPath } from "node:url";
import video from "wdio-video-reporter";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const kitRoot = path.resolve(__dirname, "../..");
const videoDir = path.join(kitRoot, "docs/media");

const binaryCandidates = [
  process.env.PHISHKIT_E2E_BIN,
  path.join(kitRoot, "target/debug/phishkit"),
  path.join(kitRoot, "target/release/phishkit"),
  path.join(
    kitRoot,
    "target/debug/bundle/macos/phishkit.app/Contents/MacOS/phishkit"
  ),
].filter(Boolean) as string[];

const application = binaryCandidates[0];

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./specs/**/*.ts"],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "tauri",
      "tauri:options": {
        application,
      },
    } as WebdriverIO.Capabilities,
  ],
  logLevel: "info",
  waitforTimeout: 20000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 1,
  framework: "mocha",
  reporters: [
    "spec",
    [
      video,
      {
        saveAllVideos: true,
        videoSlowdownMultiplier: 2,
        outputDir: videoDir,
      },
    ],
  ],
  mochaOpts: {
    ui: "bdd",
    timeout: 180000,
  },
  services: [
    [
      "@wdio/tauri-service",
      {
        driverProvider: "embedded",
      },
    ],
  ],
};
