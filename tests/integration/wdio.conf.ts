import path from "node:path";
import { fileURLToPath } from "node:url";
import video from "wdio-video-reporter";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const kitRoot = path.resolve(__dirname, "../..");
const artifactDir = path.join(__dirname, "artifacts");
const recordVideo = process.env.VIDEO === "1";

const binaryCandidates = [
  process.env.PHISHKIT_TEST_BIN,
  path.join(kitRoot, "target/debug/phishkit"),
  path.join(kitRoot, "target/release/phishkit"),
  path.join(
    kitRoot,
    "target/debug/bundle/macos/phishkit.app/Contents/MacOS/phishkit"
  ),
].filter(Boolean) as string[];

const application = binaryCandidates[0];

const reporters: WebdriverIO.Config["reporters"] = ["spec"];
if (recordVideo) {
  reporters.push([
    video,
    {
      saveAllVideos: true,
      videoSlowdownMultiplier: 2,
      outputDir: artifactDir,
    },
  ]);
}

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./specs/**/*.spec.ts"],
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
  reporters,
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
