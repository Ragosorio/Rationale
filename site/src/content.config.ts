import { defineCollection, z } from "astro:content";
import { glob } from "astro/loaders";

const docs = defineCollection({
  loader: glob({
    pattern: "**/*.md",
    base: "./src/content/docs",
    generateId: ({ entry }) => entry.replace(/\.md$/, ""),
  }),
  schema: z.object({
    lang: z.enum(["en", "es"]),
    slug: z.string(),
    title: z.string(),
    description: z.string(),
    section: z.string(),
    order: z.number().int(),
  }),
});

export const collections = { docs };
