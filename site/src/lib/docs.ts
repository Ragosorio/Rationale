export const DOC_GROUPS = [
  {
    key: "start",
    en: "Start",
    es: "Empezar",
    slugs: ["quickstart", "concepts", "prompt-master"],
  },
  {
    key: "operate",
    en: "Operate",
    es: "Operar",
    slugs: ["cli-reference", "mcp-reference", "workflow"],
  },
  {
    key: "verify",
    en: "Verify",
    es: "Verificar",
    slugs: ["versioning", "troubleshooting", "limits"],
  },
  {
    key: "evidence",
    en: "Project",
    es: "Proyecto",
    slugs: ["architecture", "agents-and-mcp", "evidence"],
  },
];

export const DOC_SLUGS = DOC_GROUPS.flatMap((group) => group.slugs);

export function docHref(lang: "en" | "es", slug: string) {
  return lang === "es" ? `/es/docs/${slug}` : `/docs/${slug}`;
}

export function docsRoot(lang: "en" | "es") {
  return lang === "es" ? "/es/docs" : "/docs";
}

export function homeHref(lang: "en" | "es") {
  return lang === "es" ? "/es/" : "/";
}
