import { sveltekit } from "@sveltejs/kit/vite";
import { imagetools } from "vite-imagetools";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
    plugins: [tailwindcss(), sveltekit(), imagetools({ force: true })],
});
