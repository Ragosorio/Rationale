# Rationale site

Astro 7 static landing and documentation site for Rationale, styled with
Tailwind CSS 4 through Tailwind's Vite plugin. The documentation uses typed
content collections and ships in English and Spanish under `/docs/*` and
`/es/docs/*`.

## Local development

```bash
npm install
npm run dev
npm run build
```

The master prompt lives at `../docs/prompt-master.md`, the same file the binary
compiles in. Both documentation locales inject it at build time: the Spanish
page shows the installed English text and explains why it is in English, so
there is no translation to drift.

The site is deployed on Vercel at
[rationale-pearl.vercel.app](https://rationale-pearl.vercel.app). Links stay
relative; set Astro's `site` option in `astro.config.mjs` if absolute canonical
URLs are ever needed.

The landing copy for both languages lives in `src/lib/landing.ts`, and the
documentation pages in `src/content/docs/{en,es}/`. Copy that describes behavior
on `main` but not yet in `releases/latest` says which release ships it; update
that wording when the release is cut.

To keep Astro and official integrations current, use the official upgrader:

```bash
npx @astrojs/upgrade
```

This site intentionally uses `@tailwindcss/vite` and
`@import "tailwindcss";`. The deprecated `@astrojs/tailwind` integration is
not used.

## Companion tool

For the complete structural-context flow, install
[codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) first:

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash
```

Codebase Memory explains where code is and how it connects. Rationale preserves
why it exists, what must survive, and who approved it. Rationale can still run
without the provider with degraded coverage.
