const translations = {
  en: {
    skip: "Skip to main content",
    navHow: "How it works",
    navProof: "Proof",
    navDocs: "Documentation",
    navInstall: "Install",
    languageLabel: "Switch language",
    themeLight: "Light mode",
    themeDark: "Dark mode",
    eyebrow: "Local causal context / pre-flight",
    heroTitle: "Give your coding agent the reason behind the code.",
    heroBody: "Rationale compiles the decisions, constraints, risks, and evidence that still govern a codebase — before an agent changes it.",
    installCta: "Install the alpha",
    quickstartCta: "Read the 5-minute quickstart",
    stateLine: "Local-first · Open source · No account · Human-approved",
    boardLabel: "Live change / synthetic demonstration",
    boardRevision: "revision 2f6d1e4",
    boardPacket: "Context packet",
    boardDiff: "Change bundle",
    boardPending: "Proposal pending",
    boardApproval: "Human approval",
    boardPacketBody: "Constraints, risks, evidence and authority before the edit.",
    boardDiffBody: "The agent changes code with the why still visible.",
    boardPendingBody: "Observed facts are captured. No decision is approved here.",
    boardApprovalBody: "A person approves, corrects, disputes, revokes or supersedes.",
    boardReady: "READY",
    boardObserved: "OBSERVED",
    boardWaiting: "WAITING",
    boardHuman: "HUMAN",
    boardFooter: "The packet travels with the change. The approval boundary stays visible.",
    bridgeEyebrow: "Three systems / one clearer boundary",
    bridgeTitle: "Structure tells you where. Git tells you what. Rationale protects why.",
    bridgeBody: "Rationale joins the tools teams already use without pretending to replace them.",
    gitQuestion: "What changed, when, and in which revision?",
    memoryQuestion: "Where is it, and how is it connected?",
    rationaleQuestion: "Why does it exist, what must survive, and who approved it?",
    worksEyebrow: "The dispatch rail",
    worksTitle: "A preflight that becomes a durable record.",
    worksBody: "The flow is deliberately split: MCP prepares and captures; the interactive CLI keeps normative decisions with a human.",
    step1: "Prepare",
    step1Body: "Capture the why before code moves.",
    step2: "Change",
    step2Body: "Make the edit with constraints in view.",
    step3: "Finalize",
    step3Body: "Bundle the diff and observed facts.",
    step4: "Review",
    step4Body: "Confirm the decision with explicit authority.",
    packetEyebrow: "The artifact",
    packetTitle: "Make the context impossible to lose.",
    packetBody: "A compact packet gives the agent the decision, the trade-offs and the evidence before it reaches for a simplification.",
    synthetic: "Synthetic demonstration",
    packetSummary: "Exponential backoff for outbound retries",
    packetWhy: "Reduces cascading failures under transient upstream errors.",
    packetDecision: "Use exponential backoff with jitter. Cap at 30s, 5 attempts.",
    packetTradeoffs: "Higher tail latency vs. improved success rate and stability.",
    packetApproval: "human-approved",
    diffTitle: "src/retry/policy.ts",
    diffBody: "The change is legible beside the reason it exists.",
    diffAdded: "export const MAX_ATTEMPTS = 5",
    diffAdded2: "export const BASE_DELAY_MS = 250",
    diffRemoved: "export const RETRY_DELAY_MS = 500",
    docsEyebrow: "Documentation / start with the work",
    docsTitle: "Everything you need to verify the claim.",
    docsBody: "Use the landing to orient yourself; use the repository docs to inspect the real behavior, limits and evidence.",
    docsStart: "Start",
    docsUnderstand: "Understand",
    docsOperate: "Operate",
    docsVerify: "Verify",
    docsQuickstart: "Five-minute quickstart",
    docsQuickstartBody: "Install, initialize a project, inspect health and run the first preflight.",
    docsArchitecture: "Factual architecture",
    docsArchitectureBody: "See the real modules and the prepare → capture → review flow.",
    docsAgents: "Agents and MCP",
    docsAgentsBody: "Connect the tools without handing approval to the protocol.",
    docsEvidence: "Dogfood and pilot evidence",
    docsEvidenceBody: "Read the evidence with its coverage, gates and uncertainty attached.",
    docsOpen: "Open documentation",
    trustEyebrow: "Trust is a product behavior",
    trustTitle: "Local by default. Explicit about uncertainty.",
    trustLocal: "What stays local",
    trustLocalBody: "The `.rationale/` canon is versioned with the project. Logs stay local. Derived SQLite/FTS can be rebuilt and is never the only copy.",
    trustHuman: "What stays human",
    trustHumanBody: "MCP can prepare and capture. It cannot approve, revoke, supersede or change authority. Those actions require `rationale review`.",
    statusEyebrow: "Release truth",
    statusTitle: "Useful now, honest about what is still open.",
    statusBody: "Rationale is pre-1.0. The core and full capture/review cycle are functional; pilot gates and human review remain visible work.",
    statusAlpha: "pre-1.0 / dogfood",
    statusEvidence: "Current public evidence: v0.0.0-dogfood.7",
    statusOpen: "Open: alpha promotion, platform matrix, provider coverage and pilot gates.",
    installEyebrow: "Install / then initialize",
    installTitle: "Protect the why before the next change.",
    installBody: "The installer places the binary locally. `rationale init` creates the project canon and detects the agent configuration.",
    cbmEyebrow: "Companion / structural coverage",
    cbmTitle: "Install Codebase Memory first.",
    cbmBody: "Codebase Memory maps where and how the code connects; Rationale carries why. The full path uses both. Rationale can still run alone with degraded coverage.",
    cbmLink: "Open Codebase Memory",
    cbmCommandLabel: "macOS + Linux",
    installCopy: "Copy command",
    copied: "Copied",
    copyFailed: "Select and copy",
    installStep2: "Then, inside the repository:",
    installStep3: "Check the project health:",
    footerLine: "Your repo. Your process. Your context.",
    footerDocs: "Docs",
    footerGithub: "GitHub",
    footerLicense: "MIT License",
    footerStatus: "No remote telemetry by default",
  },
  es: {
    skip: "Saltar al contenido principal",
    navHow: "Cómo funciona",
    navProof: "Evidencia",
    navDocs: "Documentación",
    navInstall: "Instalar",
    languageLabel: "Cambiar idioma",
    themeLight: "Modo claro",
    themeDark: "Modo oscuro",
    eyebrow: "Contexto causal local / preflight",
    heroTitle: "Dale a tu agente de código la razón detrás del código.",
    heroBody: "Rationale compila las decisiones, restricciones, riesgos y evidencia que todavía gobiernan un codebase — antes de que un agente lo cambie.",
    installCta: "Instalar el alfa",
    quickstartCta: "Leer el quickstart de 5 minutos",
    stateLine: "Local-first · Open source · Sin cuenta · Aprobación humana",
    boardLabel: "Cambio activo / demostración sintética",
    boardRevision: "revisión 2f6d1e4",
    boardPacket: "Packet de contexto",
    boardDiff: "Bundle del cambio",
    boardPending: "Propuesta pendiente",
    boardApproval: "Aprobación humana",
    boardPacketBody: "Restricciones, riesgos, evidencia y autoridad antes del cambio.",
    boardDiffBody: "El agente modifica el código con el porqué visible.",
    boardPendingBody: "Se capturan hechos observados. Aquí no se aprueba ninguna decisión.",
    boardApprovalBody: "Una persona aprueba, corrige, disputa, revoca o supersede.",
    boardReady: "LISTO",
    boardObserved: "OBSERVADO",
    boardWaiting: "ESPERA",
    boardHuman: "HUMANO",
    boardFooter: "El packet viaja con el cambio. La frontera de aprobación permanece visible.",
    bridgeEyebrow: "Tres sistemas / una frontera más clara",
    bridgeTitle: "La estructura dice dónde. Git dice qué. Rationale protege por qué.",
    bridgeBody: "Rationale conecta las herramientas que los equipos ya usan sin fingir que las reemplaza.",
    gitQuestion: "¿Qué cambió, cuándo y en qué revisión?",
    memoryQuestion: "¿Dónde está y cómo se conecta?",
    rationaleQuestion: "¿Por qué existe, qué debe sobrevivir y quién lo aprobó?",
    worksEyebrow: "El rail de despacho",
    worksTitle: "Un preflight que se vuelve un Record durable.",
    worksBody: "El flujo está separado a propósito: MCP prepara y captura; la CLI interactiva mantiene las decisiones normativas con una persona.",
    step1: "Preparar",
    step1Body: "Capturar el porqué antes de mover código.",
    step2: "Cambiar",
    step2Body: "Hacer el cambio con las restricciones a la vista.",
    step3: "Finalizar",
    step3Body: "Agrupar el diff y los hechos observados.",
    step4: "Revisar",
    step4Body: "Confirmar la decisión con autoridad explícita.",
    packetEyebrow: "El artefacto",
    packetTitle: "Haz imposible perder el contexto.",
    packetBody: "Un packet compacto le da al agente la decisión, los trade-offs y la evidencia antes de que intente simplificar algo.",
    synthetic: "Demostración sintética",
    packetSummary: "Backoff exponencial para reintentos salientes",
    packetWhy: "Reduce fallas en cascada ante errores transitorios del upstream.",
    packetDecision: "Usar backoff exponencial con jitter. Límite de 30s, 5 intentos.",
    packetTradeoffs: "Mayor latencia de cola vs. mejor tasa de éxito y estabilidad.",
    packetApproval: "aprobado por humano",
    diffTitle: "src/retry/policy.ts",
    diffBody: "El cambio se puede leer junto a la razón por la que existe.",
    diffAdded: "export const MAX_ATTEMPTS = 5",
    diffAdded2: "export const BASE_DELAY_MS = 250",
    diffRemoved: "export const RETRY_DELAY_MS = 500",
    docsEyebrow: "Documentación / empieza por el trabajo",
    docsTitle: "Todo lo necesario para verificar el claim.",
    docsBody: "Usa la landing para orientarte; usa los docs del repositorio para inspeccionar el comportamiento, los límites y la evidencia reales.",
    docsStart: "Empezar",
    docsUnderstand: "Entender",
    docsOperate: "Operar",
    docsVerify: "Verificar",
    docsQuickstart: "Quickstart de cinco minutos",
    docsQuickstartBody: "Instala, inicializa un proyecto, revisa health y ejecuta el primer preflight.",
    docsArchitecture: "Arquitectura factual",
    docsArchitectureBody: "Mira los módulos reales y el flujo prepare → capture → review.",
    docsAgents: "Agentes y MCP",
    docsAgentsBody: "Conecta las herramientas sin entregar la aprobación al protocolo.",
    docsEvidence: "Evidencia de dogfood y piloto",
    docsEvidenceBody: "Lee la evidencia con su cobertura, gates e incertidumbre adjuntas.",
    docsOpen: "Abrir documentación",
    trustEyebrow: "La confianza es comportamiento del producto",
    trustTitle: "Local por defecto. Explícito sobre la incertidumbre.",
    trustLocal: "Lo que permanece local",
    trustLocalBody: "El canon `.rationale/` se versiona con el proyecto. Los logs permanecen locales. SQLite/FTS derivado se puede reconstruir y nunca es la única copia.",
    trustHuman: "Lo que permanece humano",
    trustHumanBody: "MCP puede preparar y capturar. No puede aprobar, revocar, superseder ni cambiar autoridad. Esas acciones requieren `rationale review`.",
    statusEyebrow: "Verdad de release",
    statusTitle: "Útil ahora, honesto sobre lo que sigue abierto.",
    statusBody: "Rationale está en pre-1.0. El núcleo y el ciclo completo de captura/revisión son funcionales; los gates del piloto y la revisión humana siguen visibles.",
    statusAlpha: "pre-1.0 / dogfood",
    statusEvidence: "Evidencia pública actual: v0.0.0-dogfood.7",
    statusOpen: "Abierto: promoción del alfa, matriz de plataformas, cobertura del proveedor y gates del piloto.",
    installEyebrow: "Instalar / luego inicializar",
    installTitle: "Protege el porqué antes del próximo cambio.",
    installBody: "El instalador coloca el binario localmente. `rationale init` crea el canon del proyecto y detecta la configuración del agente.",
    cbmEyebrow: "Compañero / cobertura estructural",
    cbmTitle: "Instala Codebase Memory primero.",
    cbmBody: "Codebase Memory mapea dónde está el código y cómo se conecta; Rationale conserva el porqué. El flujo completo usa ambos. Rationale puede funcionar solo con cobertura degradada.",
    cbmLink: "Abrir Codebase Memory",
    cbmCommandLabel: "macOS + Linux",
    installCopy: "Copiar comando",
    copied: "Copiado",
    copyFailed: "Selecciona y copia",
    installStep2: "Después, dentro del repositorio:",
    installStep3: "Comprueba la salud del proyecto:",
    footerLine: "Tu repo. Tu proceso. Tu contexto.",
    footerDocs: "Docs",
    footerGithub: "GitHub",
    footerLicense: "Licencia MIT",
    footerStatus: "Sin telemetría remota por defecto",
  },
};

function boot() {
  const root = document.documentElement;
  const languageToggle = document.querySelector("[data-language-toggle]");
  const themeToggle = document.querySelector("[data-theme-toggle]");
  const copyButtons = document.querySelectorAll("[data-copy-value]");

  const savedLanguage = window.localStorage.getItem("rationale-language");
  const savedTheme = window.localStorage.getItem("rationale-theme");
  const initialLanguage = savedLanguage === "es" ? "es" : "en";
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;

  function setLanguage(language) {
    const selected = translations[language] ? language : "en";
    root.lang = selected;
    window.localStorage.setItem("rationale-language", selected);
    document.querySelectorAll("[data-i18n]").forEach((node) => {
      const key = node.dataset.i18n;
      if (translations[selected][key]) node.textContent = translations[selected][key];
    });
    document.querySelectorAll("[data-i18n-title]").forEach((node) => {
      const key = node.dataset.i18nTitle;
      if (translations[selected][key]) node.title = translations[selected][key];
    });
    if (languageToggle) {
      languageToggle.textContent = selected === "en" ? "ES" : "EN";
      languageToggle.setAttribute("aria-label", translations[selected].languageLabel);
    }
    document.title = selected === "en"
      ? "Rationale — Context behind the code"
      : "Rationale — El contexto detrás del código";
  }

  function setTheme(theme) {
    const selected = theme === "dark" ? "dark" : "light";
    root.dataset.theme = selected;
    window.localStorage.setItem("rationale-theme", selected);
    if (themeToggle) {
      const key = selected === "dark" ? "themeLight" : "themeDark";
      themeToggle.setAttribute("aria-label", translations[root.lang][key]);
      themeToggle.setAttribute("aria-pressed", String(selected === "dark"));
      themeToggle.querySelector("[data-theme-icon]").textContent = selected === "dark" ? "☼" : "◐";
    }
  }

  setLanguage(initialLanguage);
  setTheme(savedTheme || (prefersDark ? "dark" : "light"));

  languageToggle?.addEventListener("click", () => setLanguage(root.lang === "en" ? "es" : "en"));
  themeToggle?.addEventListener("click", () => setTheme(root.dataset.theme === "dark" ? "light" : "dark"));

  copyButtons.forEach((button) => {
    button.addEventListener("click", async () => {
      const value = button.dataset.copyValue;
      const status = button.querySelector("[data-copy-status]");
      try {
        await navigator.clipboard.writeText(value);
        status.textContent = translations[root.lang].copied;
      } catch {
        status.textContent = translations[root.lang].copyFailed;
      }
      window.setTimeout(() => {
        status.textContent = translations[root.lang].installCopy;
      }, 1800);
    });
  });
}

export { boot };
