import { fileURLToPath } from "url";
import { dirname, resolve } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default {
  extensions: [".md"],
  // smartypants: {
  //   dashes: "oldschool",
  // },
  // remarkPlugins: [require("remark-abbr")],
  // rehypePlugins: [
  //   require("rehype-slug"),
  //   [
  //     require("rehype-autolink-headings"),
  //     {
  //       behavior: "wrap",
  //     },
  //   ],
  // ],
  layout: {
    _: resolve(__dirname, "./src/routes/blog/_layout.svelte"),
    blog: resolve(__dirname, "./src/routes/blog/_layout.svelte"),
    projects: resolve(__dirname, "./src/routes/projects/_layout.svelte"),
  },
};
