/** ESLint Flat Config：统一前端（TS/Svelte）代码质量规则。 */

import js from "@eslint/js";
import eslintConfigPrettier from "eslint-config-prettier";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import globals from "globals";
import svelte from "eslint-plugin-svelte";

export default [
  {
    ignores: ["node_modules/**", "build/**", "dist/**", ".svelte-kit/**", "src-tauri/target/**", "target/**"],
  },
  js.configs.recommended,
  {
    files: ["**/*.{js,ts}"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      parser: tsParser,
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
    },
    rules: {
      ...tsPlugin.configs.recommended.rules,
      "no-undef": "off",
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          ignoreRestSiblings: true,
        },
      ],
    },
  },
  ...svelte.configs["flat/recommended"],
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parserOptions: {
        parser: tsParser,
      },
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
    },
    rules: {
      "no-undef": "off",
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          ignoreRestSiblings: true,
        },
      ],
      "svelte/valid-compile": "warn",
    },
  },
  eslintConfigPrettier,
];
