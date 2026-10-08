import path from "node:path";
import { fileURLToPath } from "node:url";
import type { NextConfig } from "next";

const appRoot = path.dirname(fileURLToPath(import.meta.url));
const uiEntry =
  process.env.KAI_UI_DIST === "1"
    ? path.join(appRoot, "../dist/index.js")
    : path.join(appRoot, "../src/index.ts");

const nextConfig: NextConfig = {
  outputFileTracingRoot: appRoot,
  turbopack: {
    resolveAlias: {
      "@felipechandiadev/ui": uiEntry,
    },
  },
  webpack: (config) => {
    config.resolve.alias = {
      ...config.resolve.alias,
      "@felipechandiadev/ui$": uiEntry,
    };
    return config;
  },
};

export default nextConfig;
