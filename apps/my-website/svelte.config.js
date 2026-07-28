import { mdsvex } from "mdsvex";
import mdsvexConfig from "./mdsvex.config.js";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import staticAdapter from "@sveltejs/adapter-static";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  extensions: [".svelte", ...mdsvexConfig.extensions],
  preprocess: [
    mdsvex(mdsvexConfig),
    vitePreprocess(),
  ],

  kit: {
    adapter: staticAdapter(),
    prerender: {
      entries: ["*", "/sitemap.xml", "/blog/getSearchIndex.json"],
    },
    alias: {
      $components: "./src/components",
      $utils: "./src/utils",
      $icons: "./src/icons",
      $types: "./src/types.d.ts",
    },
  },
};

export default config;
