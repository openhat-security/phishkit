import { defineConfig } from "vitepress";

const repo = "https://github.com/openhat-security/phishkit";

export default defineConfig({
  title: "phishkit",
  titleTemplate: ":title · phishkit",
  description:
    "An authorized end-to-end phishing assessment platform: evilginx AiTM plus a native email campaign engine in one desktop app.",
  lang: "en-US",
  // Published under a repository subpath on GitHub Pages.
  base: "/phishkit/",
  cleanUrls: true,
  lastUpdated: true,
  ignoreDeadLinks: false,
  sitemap: {
    hostname: "https://openhat-security.github.io/phishkit/",
  },
  head: [
    ["meta", { name: "theme-color", content: "#dc2626" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:title", content: "phishkit" }],
    [
      "meta",
      {
        property: "og:description",
        content:
          "Authorized AiTM + awareness assessments, end to end, in one desktop app.",
      },
    ],
  ],

  themeConfig: {
    nav: [
      { text: "Guide", link: "/guide/", activeMatch: "/guide/" },
      {
        text: "Security",
        items: [
          { text: "Authorized use", link: "/guide/authorized-use" },
          { text: "Threat model", link: "/reference/threat-model" },
          { text: "Privacy", link: "/reference/privacy" },
          {
            text: "Local data and network activity",
            link: "/reference/data-and-network",
          },
          { text: "Platform support", link: "/reference/platform-support" },
          { text: "Security policy", link: `${repo}/blob/main/SECURITY.md` },
        ],
      },
      {
        text: "Reference",
        link: "/reference/architecture",
        activeMatch: "/reference/",
      },
      { text: "Install", link: "/guide/install" },
    ],

    sidebar: {
      "/guide/": [
        {
          text: "Getting started",
          items: [
            { text: "What phishkit is", link: "/guide/" },
            { text: "Authorized use", link: "/guide/authorized-use" },
            { text: "Install", link: "/guide/install" },
            { text: "Quick start", link: "/guide/quick-start" },
            { text: "Walkthrough videos", link: "/guide/walkthrough" },
            { text: "Testing", link: "/guide/testing" },
          ],
        },
        {
          text: "Run an assessment",
          items: [
            { text: "Campaign guide", link: "/guide/campaigns" },
            { text: "Phishlet authoring", link: "/guide/phishlets" },
            { text: "Command line", link: "/guide/cli" },
          ],
        },
      ],
      "/reference/": [
        {
          text: "Reference",
          items: [
            { text: "Architecture", link: "/reference/architecture" },
            { text: "Platform support", link: "/reference/platform-support" },
            {
              text: "Local data and network activity",
              link: "/reference/data-and-network",
            },
            { text: "Threat model", link: "/reference/threat-model" },
            { text: "Privacy", link: "/reference/privacy" },
            { text: "Release process", link: "/reference/release" },
          ],
        },
        {
          text: "Project",
          items: [
            { text: "Changelog", link: `${repo}/blob/main/CHANGELOG.md` },
            { text: "Security policy", link: `${repo}/blob/main/SECURITY.md` },
            { text: "Contributing", link: `${repo}/blob/main/CONTRIBUTING.md` },
            {
              text: "Code of conduct",
              link: `${repo}/blob/main/CODE_OF_CONDUCT.md`,
            },
          ],
        },
      ],
    },

    socialLinks: [{ icon: "github", link: repo }],

    editLink: {
      pattern: `${repo}/edit/main/docs/:path`,
      text: "Edit this page on GitHub",
    },

    search: { provider: "local" },

    footer: {
      message: "GPL-3.0. For authorized security assessments only.",
      copyright: "© Openhat Security",
    },
  },
});
