---
title: "Adding Prettier to a React App initialized with Create React App (CRA)"
description: "Adding Prettier to a React App initialized with Create React App (CRA) and then configuring ESLint and Prettier to run automatically in VSCode"
author: "Saikat Das"
tags:
  - react
  - reactjs
  - create-react-app
  - cra
  - eslint
  - prettier
icon: "react.png"
date: "2020-04-05"
---

Whenever I bootstrap a new React project, I reach for **Create React App** unless I need static pages, in which case I use **GatsbyJS**. There's a case to be made here for **NextJS** but I haven't needed it yet, so, that's a post for another day.

<script>
  import ShruggingImage from "./shrugging.webp";
</script>

<img src={ShruggingImage} alt="Shrugging" class="mx-auto">


The first thing I do after setting up any JS project is setup linting and formatting using [Prettier](https://prettier.io/docs/en/install.html) and [ESLint](https://eslint.org/docs/developer-guide/development-environment). And then tweak a few things here and there in the ESLint config file (`.eslintrc`).

I use [Visual Studio Code](https://code.visualstudio.com/) as my editor of choice and it has plugins available for [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint) and [Prettier](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode). The ESLint plugin brings the linting capabilities directly in your code editor instead of the command line and the Prettier plugin is similar in that it formats your code in the editor itself instead of having to run a terminal command.

React apps bootstrapped with _Create React App_ already ship with ESLint integrated and the linting errors show up in the terminal console and in the browser console. If you have the ESLint plugin, the linting errors also show up in the code editor. What I add to this is Prettier to format my code on save just the way I like it and make it work in tandem with ESLint.

The Create React App documentation [glosses over](https://create-react-app.dev/docs/setting-up-your-editor/#displaying-lint-output-in-the-editor) this but does not go over in detail how to set it up. Here are the minimal steps to setting it up without struggling with the existing tooling.

## Steps

1. Bootstrap a project with Create React App

```bash
$ npx create-react-app <project-name>
```

2. Open the project with VS Code and open the built-in terminal
3. Install prettier

```bash
$ npm i prettier
```

4. Install the ESLint and Prettier configuration as instructed [here](https://prettier.io/docs/en/integrating-with-linters.html#recommended-configuration)

```bash
$ npm i eslint-config-prettier eslint-plugin-prettier
```

5. Edit the `"eslintConfig"` section in `package.json` or create a `.eslintrc` file at the project root to hold our ESLint configuration.

```json
{
  "extends": ["react-app", "plugin:prettier/recommended"],
  "rules": {}
}
```

6. Optionally, add a `.prettierrc` file at the project root to hold our Prettier configuration. I like all the default Prettier provides except trailing commas which I configure.

```json
{
  "trailingComma": "all"
}
```

You can play around with Prettier's possible configuration options [here](https://prettier.io/playground/) and then copy your config from the playground into your config file
