import js from "@eslint/js";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import sveltePlugin from "eslint-plugin-svelte";
import svelteParser from "svelte-eslint-parser";
import globals from "globals";

export default [
    // Base JS recommended rules
    js.configs.recommended,

    // TypeScript-eslint flat/recommended (sets up plugin + parser for TS/JS)
    ...tsPlugin.configs["flat/recommended"],

    // Svelte flat/recommended (sets up svelte-eslint-parser for .svelte files)
    ...sveltePlugin.configs["flat/recommended"],

    // Project-wide rule overrides
    {
        rules: {
            // This site has no base path configured, so the resolve() rule is N/A
            "svelte/no-navigation-without-resolve": "off",
        },
    },

    // TypeScript + JS files: add browser/node globals
    {
        files: ["**/*.ts", "**/*.js"],
        languageOptions: {
            parser: tsParser,
            parserOptions: {
                sourceType: "module",
                ecmaVersion: 2019,
            },
            globals: {
                ...globals.browser,
                ...globals.es2017,
                ...globals.node,
            },
        },
    },

    // Svelte files: wire TypeScript as the inner parser for <script lang="ts">
    {
        files: ["**/*.svelte"],
        languageOptions: {
            parser: svelteParser,
            parserOptions: {
                parser: tsParser,
                sourceType: "module",
                ecmaVersion: 2019,
            },
            globals: {
                ...globals.browser,
                ...globals.es2017,
                ...globals.node,
            },
        },
    },

    // Global ignores
    {
        ignores: ["*.cjs", "build/**", ".svelte-kit/**"],
    },
];
