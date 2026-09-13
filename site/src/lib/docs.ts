export const DOC_GROUPS = [
  {
    key: "start",
    en: "Start",
    es: "Empezar",
    slugs: ["quickstart", "concepts", "prompt-master", "skill"],
  },
  {
    key: "operate",
    en: "Operate",
    es: "Operar",
    slugs: ["workflow", "control-room", "agents-and-mcp", "cli-reference", "mcp-reference"],
  },
  {
    key: "verify",
    en: "Verify",
    es: "Verificar",
    slugs: ["versioning", "troubleshooting", "limits"],
  },
  {
    key: "project",
    en: "Project",
    es: "Proyecto",
    slugs: ["architecture", "evidence"],
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
