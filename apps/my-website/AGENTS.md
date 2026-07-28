# AGENTS.md

This is a living document for agents (and developers) working in this repository. It captures the conventions, patterns, and assumptions specific to this codebase. Update this file whenever overarching changes are made that affect how agents should operate here — keep it current with the actual state of the repo, not as a changelog.

---

## Handoff

Before starting any work, check whether a `HANDOFF.md` file exists at the repository root.

- **If it exists:** Read it fully, absorb the context and next steps described in it, then **delete the file** before proceeding.
- **If it does not exist:** No handoff is needed — proceed normally.

If you are stopping mid-task and want to preserve state for the next agent, create a `HANDOFF.md` at the repository root describing exactly where you left off and what the next steps are. Remove it once the work is complete.

---

## Project Overview

This is a personal website and blog for Saikat Das, built as a **statically generated site** with the following core stack:

| Technology | Role |
|---|---|
| [SvelteKit 2](https://kit.svelte.dev/) | Framework (static adapter) |
| [Svelte 5](https://svelte.dev/) | UI framework (legacy compat syntax in use) |
| [TypeScript](https://www.typescriptlang.org/) | Language |
| [TailwindCSS v4](https://tailwindcss.com/) | Styling (CSS-based config via `@import "tailwindcss"`) |
| [mdsvex 0.12](https://mdsvex.com/) | Markdown-in-Svelte for blog posts and project entries |
| [PostCSS](https://postcss.org/) + autoprefixer + cssnano | CSS processing |
| [lunr](https://lunrjs.com/) | Client-side full-text search index |
| [date-fns v4](https://date-fns.org/) | Date formatting |
| [vite-imagetools v10](https://github.com/JonasKruckenberg/imagetools) | Image optimization at build time |
| [Vite 7](https://vitejs.dev/) | Build tool |

The static adapter renders all pages at build time. There is no server-side runtime.

---

## Repository Structure

```
/
├── mdsvex.config.js          # mdsvex configuration (extensions, absolute layout paths)
├── svelte.config.js          # SvelteKit config (adapter, path aliases via kit.alias)
├── vite.config.ts            # Vite config (sveltekit + imagetools plugins)
├── postcss.config.cjs        # PostCSS plugins (@tailwindcss/postcss)
├── eslint.config.js          # ESLint flat config (replaces .eslintrc.cjs)
├── tsconfig.json             # TypeScript config (extends .svelte-kit/tsconfig.json)
├── src/
│   ├── app.html              # Root HTML shell (%sveltekit.head%, %sveltekit.body%)
│   ├── app.postcss           # Global styles (@import "tailwindcss" + Prism theme)
│   ├── types.d.ts            # Shared TypeScript types
│   ├── global.d.ts           # Global ambient declarations
│   ├── components/           # Reusable Svelte components
│   ├── icons/                # SVG icon components (Svelte wrappers)
│   ├── routes/               # SvelteKit file-based routes (SK2 conventions)
│   └── utils/                # Utility functions and constants
└── static/                   # Static assets (images, logos, manifest, service worker)
```

---

## Path Aliases

Defined in `svelte.config.js` under `kit.alias`. Always use these instead of relative paths where applicable:

| Alias | Resolves to |
|---|---|
| `$components` | `./src/components` |
| `$utils` | `./src/utils` |
| `$icons` | `./src/icons` |

Example import:
```ts
import BlogPostCard from "$components/BlogPostCard/BlogPostCard.svelte";
import { WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT } from "$utils/constants";
import GithubIcon from "$icons/logos/github-icon.svelte";
```

---

## Component Conventions

- Components live in `src/components/<ComponentName>/<ComponentName>.svelte`.
- Use `<script lang="ts">` in all Svelte components.
- Props are `export let` declarations with explicit TypeScript types.
- Scoped styles go in a `<style>` block inside the component. Tailwind utility classes are preferred for layout; scoped styles are used for custom CSS-only effects (e.g., animations, custom decorations).
- The `class` prop on icons is named `className` (not `class`) to avoid conflicts with the JS keyword.

---

## Icon Components

Icons are thin Svelte wrappers around inline SVGs, organized under `src/icons/`:

```
src/icons/
├── flat-color-icons/   # Flat UI icons (home, news, deployment, search, share)
├── logos/              # Brand/service logos (GitHub, LinkedIn, npm, etc.)
└── twemoji/            # Twemoji emoji SVGs
```

Each icon accepts a `className` prop (default `"h-4"`) to control size via Tailwind height utilities.

---

## Routes and Layouts

SvelteKit 2 file-based routing is used. Key conventions:

- `+layout.svelte` — top-level site layout (navigation, global wrapper)
- `_layout.svelte` inside a sub-route — mdsvex layout for blog posts / project entries (prefixed `_` so SK2 ignores it as a route)
- Routes prefixed with `_` (like `_index/`) are non-routable component directories, not pages
- API endpoints use `<name>/+server.ts` subdirectory format with `GET` export and `json()` from `@sveltejs/kit`
- Page data loading uses `+page.ts` co-located load functions with `export let data` prop in `+page.svelte`
- `src/routes/+layout.ts` exports `export const prerender = true` to prerender all routes at build time

---

## Blog Posts

Blog posts live at:
```
src/routes/blog/posts/<slug>/+page.md
```

The `+page.md` file uses mdsvex and **must** include this frontmatter:

```yaml
---
title: "Post Title"
description: "Short description"
author: "Saikat Das"
tags:
  - tag1
  - tag2
icon: "icon-filename.png"   # filename under static/assets/icons/post-topics/
date: "YYYY-MM-DD"
---
```

- Layout is automatically applied via `mdsvex.config.js` (maps to `src/routes/blog/_layout.svelte`).
- Posts are discovered at build time via `import.meta.glob` in `src/routes/blog/getBlogPosts.json/+server.ts`.
- Sorted newest-first by `date`.
- Slugs are derived from the directory path (e.g., `blog/posts/react-hooks-an-overview`).

---

## Project Entries

Project entries live at:
```
src/routes/projects/entries/<slug>/+page.md
```

Required frontmatter:

```yaml
---
title: "Project Title"
description: "Short description"
technologiesUsed:
  - Technology1
  - Technology2
author: "Saikat Das"
coverPhoto: "<slug>/cover-image.jpg"   # relative to static/assets/images/routes/
startDate: "YYYY-MM-DD"
endDate: "YYYY-MM-DD"
layout: "projects"
---
```

- Layout is mapped to `src/routes/projects/_layout.svelte` via the `layout: "projects"` frontmatter key.
- Entries are discovered via `import.meta.glob` in `src/routes/projects/getProjectEntries.json/+server.ts`.
- Sorted newest-first by `startDate`.

---

## Styling

- **TailwindCSS v4** with CSS-based config. No `tailwind.config.cjs` — configuration is done via `@import "tailwindcss"` and `@plugin` directives in `src/app.postcss`.
- Global styles in `src/app.postcss` (imported in the root layout). This file includes the Tailwind import, typography plugin, and the Prism **Synthwave '84** syntax highlighting theme for code blocks.
- Extended theme colors: `emerald` and `fuchsia` from Tailwind's color palette.
- Typography plugin (`@tailwindcss/typography`) is enabled via `@plugin "@tailwindcss/typography"`; prose styles use `prose-emerald`.
- The `code::before` and `code::after` content is suppressed globally to avoid backtick artifacts around inline code.

---

## TypeScript

- Shared types are in `src/types.d.ts`:
  - `BlogPostCardDetails` — shape of a blog post card (title, description, tags, created, icon, slug)
  - `ProjectEntryCardDetails` — shape of a project entry card (title, description, technologiesUsed, coverPhoto, startDate, endDate, slug)
- ESLint uses `@typescript-eslint/parser` and `eslint-plugin-svelte` (flat config in `eslint.config.js`).
- `tsconfig.json` extends `.svelte-kit/tsconfig.json` and is at the root.

---

## Build and Development

```bash
# Install dependencies
npm install

# Start dev server
npm run dev

# Production build (static)
npm run build

# Preview the production build
npm run preview

# Type-check Svelte files
npm run check
```

`@sveltejs/adapter-static` v3 outputs flat `.html` files directly (e.g., `build/404.html`, `build/blog.html`). No postbuild scripts are needed.

---

## SEO

- The `SEOMetaHeader` component (`src/components/SEO/SEOMetaHeader.svelte`) is used on listing pages.
- Individual blog and project pages render their own `<svelte:head>` with full Open Graph and Twitter Card meta tags inside their respective `_layout.svelte` files.
- A `sitemap.xml` is generated at build time via `src/routes/sitemap.xml/+server.ts`.
- A `manifest.webmanifest` and service worker are included in `static/` for PWA support.

---

## Dev Container

The `.devcontainer/devcontainer.json` uses the `mcr.microsoft.com/devcontainers/base:noble` (Ubuntu Noble) image with the following features:

| Feature | Purpose |
|---|---|
| `ghcr.io/devcontainers/features/node:1` | Node.js runtime |
| `ghcr.io/devcontainers/features/github-cli:1` | `gh` CLI |
| `ghcr.io/devcontainers-extra/features/ansible:2` | Ansible + ansible-vault |

The `postCreateCommand` runs `.devcontainer/postCreate.sh`, which: checks the SSH agent, configures `gh` to use SSH, and — if `ansible/.vault_pass` exists — automatically runs `ansible-playbook ansible/setup_env.yml` to generate `.env` from the vault.

---

## Hosting & DNS

The site is deployed to **GitHub Pages** via the `Deploy to GitHub Pages` GitHub Actions workflow (`.github/workflows/deploy.yml`). The build runs inside the devcontainer using `devcontainers/ci`, so CI exactly mirrors the local environment.

| Aspect | Detail |
|---|---|
| Static output | `build/` directory produced by `npm run build` |
| Pages source | GitHub Actions workflow (`build_type: workflow`) |
| Custom domain | `saikat.dev` (configured via `static/CNAME`) |
| DNS provider | Cloudflare (orange cloud — proxied) |
| SSL mode | Cloudflare Full (Cloudflare ↔ visitor HTTPS; Cloudflare ↔ GitHub HTTPS without strict cert validation) |
| www redirect | `www.saikat.dev` CNAME → `saikatdas0790.github.io` (proxied) |

**DNS records** (all proxied through Cloudflare):
- 4× A records: `saikat.dev` → GitHub Pages IPs (`185.199.108-111.153`)
- 4× AAAA records: `saikat.dev` → GitHub Pages IPv6 IPs (`2606:50c0:800{0-3}::153`)
- 1× CNAME: `www.saikat.dev` → `saikatdas0790.github.io`

**Devcontainer image** is cached to GHCR at `ghcr.io/saikatdas0790/my-website-devcontainer` to speed up CI builds.

---

## Secrets & Ansible Vault

Cloudflare credentials are managed with Ansible Vault. The encrypted `ansible/vars/vault.yml` is committed; the vault password lives only in `ansible/.vault_pass` (gitignored) and the `ANSIBLE_VAULT_PASSWORD` GitHub Actions secret.

**First-time setup (new developer):**
```bash
echo 'your_vault_password' > ansible/.vault_pass && chmod 600 ansible/.vault_pass
```
`postCreate.sh` will then automatically run the playbook to generate `.env` on next container start.

**Stored secrets** (only truly sensitive values go in the vault):

| Variable | Where |
|---|---|
| `CLOUDFLARE_API_TOKEN` | Vault (`vault_cloudflare_api_token`) |
| `CLOUDFLARE_ZONE_ID` | Plaintext in `vars/main.yml` |
| `CLOUDFLARE_ACCOUNT_ID` | Plaintext in `vars/main.yml` |

The API token (named `my-website` in Cloudflare) has the following permissions:
- `Account → Cloudflare Pages: Edit`
- `Zone → DNS: Edit`
- `Account → Workers Scripts: Edit`

**Regenerate `.env` after editing secrets:**
```bash
cd ansible && ansible-playbook setup_env.yml
```

**CI:** The `ANSIBLE_VAULT_PASSWORD` secret is stored in GitHub Actions but the current deploy workflow does not use it — the build only needs Node.js. It is available for future workflows that need Cloudflare API access (e.g., cache purging, DNS updates). See `ansible/README.md` for full documentation.

---

## Updating This File

Update `AGENTS.md` when any of the following change:

- Path aliases are added, removed, or renamed
- New content types are introduced (new frontmatter schemas, new route patterns)
- The tech stack changes (dependencies swapped, new frameworks added)
- New conventions are established for components, icons, or utilities
- Build or development commands change
- Dev container configuration changes significantly

Do **not** use this file as a changelog. It should always reflect the current state of the repository, not a history of changes.
