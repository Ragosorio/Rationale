import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

// Astro 7 + Tailwind 4: use Tailwind's Vite plugin, not the deprecated @astrojs/tailwind integration.
export default defineConfig({
  vite: {
    plugins: [tailwindcss()],
  },
});
