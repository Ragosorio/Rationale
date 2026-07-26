# Rationale landing

Astro 7 static landing page for Rationale, styled with Tailwind CSS 4 through
Tailwind's Vite plugin.

## Local development

```bash
npm install
npm run dev
npm run build
```

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
