import { existsSync, readFileSync, readdirSync } from "node:fs";
import { extname, join } from "node:path";

const dist = new URL("../dist/", import.meta.url);

function fail(message) {
  throw new Error(`static site check failed: ${message}`);
}

function read(relativePath) {
  const path = new URL(relativePath, dist);
  if (!existsSync(path)) fail(`missing ${relativePath}`);
  return readFileSync(path, "utf8");
}

function requireText(content, text, context) {
  if (!content.includes(text)) fail(`${context} does not contain ${JSON.stringify(text)}`);
}

function filesUnder(relativeDirectory) {
  const root = new URL(relativeDirectory, dist);
  const files = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const relativePath = join(relativeDirectory, entry.name);
    if (entry.isDirectory()) files.push(...filesUnder(`${relativePath}/`));
    else files.push(relativePath);
  }
  return files;
}

const english = read("index.html");
const spanish = read("es/index.html");

requireText(english, '<html lang="en"', "English landing");
requireText(spanish, '<html lang="es"', "Spanish landing");
requireText(english, 'href="/docs/"', "English documentation navigation");
requireText(spanish, 'href="/es/docs/"', "Spanish documentation navigation");
requireText(english, 'href="#quick-start"', "English hero");
requireText(spanish, 'href="#quick-start"', "Spanish hero");
requireText(english, 'id="quick-start"', "English quick start");
requireText(spanish, 'id="quick-start"', "Spanish quick start");
requireText(english, "/rationale-preflight", "English Claude Code actions");
requireText(spanish, "/rationale-preflight", "Spanish Claude Code actions");
requireText(english, "Prepare this change with Rationale", "English Codex request");
requireText(spanish, "Prepara este cambio con Rationale", "Spanish Codex request");

const builtFiles = filesUnder("");
const browserText = builtFiles
  .filter((path) => [".html", ".js"].includes(extname(path)))
  .map(read)
  .join("\n");
for (const forbidden of ["data-i18n", "setLanguage", "rationale-language"]) {
  if (browserText.includes(forbidden)) fail(`browser bundle contains ${forbidden}`);
}

const css = builtFiles
  .filter((path) => extname(path) === ".css")
  .map(read)
  .join("\n");
if (!/@view-transition\s*\{\s*navigation\s*:\s*auto/.test(css)) {
  fail("CSS does not enable native multi-page View Transitions");
}

for (const htmlPath of builtFiles.filter((path) => extname(path) === ".html")) {
  const html = read(htmlPath);
  for (const match of html.matchAll(/href="(\/[^"#?]*)[^"]*"/g)) {
    const href = match[1];
    if (href.startsWith("//") || extname(href)) continue;
    const destination =
      href === "/" ? "index.html" : `${href.replace(/^\/|\/$/g, "")}/index.html`;
    if (!existsSync(new URL(destination, dist))) {
      fail(`${htmlPath} links to missing ${href}`);
    }
  }
}

console.log(`Static site check passed (${builtFiles.filter((path) => extname(path) === ".html").length} HTML pages).`);
