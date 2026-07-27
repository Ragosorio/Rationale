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

The canonical agent prompts live at `../docs/prompt-master.md` and
`../docs/prompt-master.es.md`. The landing imports both at build time, and the
documentation prompt pages render the matching source for their locale.

The site currently uses relative canonical URLs because the production Vercel
hostname has not been recorded in this repository yet. Set Astro's `site`
option in `astro.config.mjs` when that hostname is confirmed.

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
