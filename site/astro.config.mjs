import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

// Astro 7 + Tailwind 4: use Tailwind's Vite plugin, not the deprecated @astrojs/tailwind integration.
export default defineConfig({
  markdown: {
    // Sin color por defecto Shiki no escribe un fondo inline: el bloque usa los
    // tokens del sitio y el color de sintaxis sigue al tema elegido.
    shikiConfig: {
      themes: { light: "github-light", dark: "github-dark" },
      defaultColor: false,
    },
  },
  vite: {
    plugins: [tailwindcss()],
  },
});
