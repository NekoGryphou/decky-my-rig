import react from "@vitejs/plugin-react";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { defineConfig } from "vitest/config";
import type { Plugin } from "vite";

interface Manifest {
  name: string;
}

const manifest = JSON.parse(
  readFileSync(new URL("./plugin.json", import.meta.url), "utf8"),
) as Manifest;

function deckyManifest(): Plugin {
  const publicId = "@decky/manifest";
  const resolvedId = `\0${publicId}`;
  return {
    name: "decky-manifest",
    resolveId: (id) => (id === publicId ? resolvedId : undefined),
    load: (id) =>
      id === resolvedId
        ? `export default ${JSON.stringify({ name: manifest.name })};`
        : undefined,
  };
}

/** The folder of the package that `id`, a bundled module, belongs to. */
function packageFolder(id: string): string | undefined {
  return /^(.*[\\/]node_modules[\\/](?:@[^\\/]+[\\/])?[^\\/]+)[\\/]/.exec(
    id,
  )?.[1];
}

/** Writes the license texts of every package bundled into the plugin. */
function thirdPartyNotices(): Plugin {
  return {
    name: "third-party-notices",
    generateBundle(_options, bundle) {
      const folders = new Set<string>();
      for (const output of Object.values(bundle)) {
        if (output.type !== "chunk") continue;
        for (const id of output.moduleIds) {
          const folder = packageFolder(id);
          if (folder) folders.add(folder);
        }
      }
      let notices = "Decky My Rig frontend dependency notices\n";
      for (const folder of [...folders].sort()) {
        const meta = JSON.parse(
          readFileSync(join(folder, "package.json"), "utf8"),
        ) as { name: string; version: string; license?: string };
        const files = readdirSync(folder)
          .filter((file) => /^(LICEN[CS]E|COPYING|NOTICE)/i.test(file))
          .sort();
        if (!meta.license || files.length === 0) {
          this.error(`missing license evidence: ${meta.name} ${meta.version}`);
        }
        notices += `\n=== ${meta.name} ${meta.version} ===\nLicense expression: ${meta.license}\n`;
        for (const file of files) {
          notices += `\n--- ${file} ---\n${readFileSync(join(folder, file), "utf8")}\n`;
        }
      }
      this.emitFile({
        type: "asset",
        fileName: "THIRD-PARTY-NOTICES.txt",
        source: notices,
      });
    },
  };
}

export default defineConfig({
  plugins: [deckyManifest(), react(), thirdPartyNotices()],
  build: {
    lib: {
      entry: "src/index.tsx",
      name: "DeckyMyRig",
      formats: ["iife"],
      fileName: () => "index.js",
    },
    target: "es2020",
    minify: true,
    rolldownOptions: {
      external: ["react", "react-dom", "react/jsx-runtime", "@decky/ui"],
      output: {
        exports: "named",
        // ? Decky imports the global-backed IIFE as an ES module.
        postFooter: "export default DeckyMyRig.default;",
        globals: {
          react: "SP_REACT",
          "react-dom": "SP_REACTDOM",
          "react/jsx-runtime": "SP_JSX",
          "@decky/ui": "DFL",
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    restoreMocks: true,
    clearMocks: true,
    include: ["src/**/*.test.tsx"],
  },
});
