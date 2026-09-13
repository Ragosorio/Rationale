export const landingCopy = {
  en: {
    skip: "Skip to main content",
    navLoop: "The loop",
    navStart: "Get started",
    navSkill: "Skill",
    navControlRoom: "Control Room",
    navDocs: "Docs",
    githubLabel: "Rationale on GitHub",
    languageLabel: "Leer en español",
    themeLabel: "Dark theme",
    brandTagline: "Causal memory for coding agents",

    heroTitleA: "Your agents forget why.",
    heroTitleB: "Rationale remembers.",
    heroBody:
      "Before an agent changes your code, Rationale hands it the rules, decisions and relationships that govern that code. After the change, what stays true is written back to your repository automatically, and the rules you pin stay out of any agent's reach.",
    startCta: "Get started",
    quickstartCta: "Quickstart guide",
    heroMeta: "v1.0.0 stable · Git and Codebase Memory · no account · no telemetry · 127.0.0.1",

    problemTitle: "You explained the rule. The next chat never heard it.",
    problemMonday: "Monday, you",
    problemThursday: "Thursday, a fresh agent",
    problemQuote: "“Never retry a payment that already reached the bank.”",
    problemCommit: "commit: cleanup",
    problemBody:
      "It works in Monday’s conversation. On Thursday a fresh agent sees only the code, removes the guard and calls it a cleanup.",
    problemAnswer:
      "Rationale keeps the rule next to the code it governs and hands it to every agent before that code is touched.",

    loopTitle: "Context before the change. Memory after it.",
    loopBody: "There is no approval queue. Agents capture what they learn; you keep authority over what must not move.",
    loopSteps: [
      { tool: "Codebase Memory", code: false, title: "Locate", body: "Find the symbol, its callers and the files around it." },
      {
        tool: "prepare_change",
        code: true,
        title: "Prepare",
        body: "Governing rules, explained relationships, the structural neighborhood and the target’s code, within a token ceiling.",
      },
      { tool: "your tests", code: false, title: "Change", body: "The smallest change consistent with that context." },
      {
        tool: "finalize_change",
        code: true,
        title: "Capture",
        body: "Durable knowledge becomes canonical Records in the same call. Noise and duplicates are discarded with a reason.",
      },
      {
        tool: "rationale pin",
        code: true,
        title: "Pin",
        body: "Fix the rules no agent may replace. Any attempt becomes a conflict only you resolve.",
      },
    ],

    startTitle: "Up and running in five minutes.",
    startBody:
      "Install once per machine, initialize once per repository, then keep working the way you already do. Updating or uninstalling never touches `.rationale/`.",
    startCompanionA: "Install",
    startCompanionB: " (recommended)",
    startInstall: "Install Rationale on macOS or Linux",
    startInstallNote: "The installer verifies checksums, uses the stable channel and registers the MCP server for the agents it finds.",
    startInit: "Initialize your repository",
    startInitNote: "Creates the canon in `.rationale/` and writes the protocol into `CLAUDE.md`, `AGENTS.md` or the Cursor rule.",
    startRestart: "Restart your agent and check the connection",
    startRestartNote: "In Claude Code run `/rationale-health`. In Codex ask “Check Rationale health.”",
    startAsk: "Ask for a real change",
    startAskQuote: "“Make payment retries stop after three attempts.”",
    startAskNote: "You don’t have to mention Rationale. The agent prepares the change, makes it, and captures what stays true.",
    startWatch: "Watch it live, and pin what must not move",
    startWindows: "On Windows, the PowerShell installer is in the quickstart.",
    copyLabel: "Copy",

    skillTitle: "One skill teaches your agent the whole job.",
    skillBody:
      "The protocol in `CLAUDE.md` and `AGENTS.md` tells an agent to prepare and capture. The `rationale` skill shows it how. A short router loads only the playbook for the operation at hand, so the depth costs nothing until a task needs it.",
    skillFeatures: [
      {
        title: "Picked up when it applies",
        body: "Claude Code and Codex can load it on their own when a task matches. `/rationale capture` or `$rationale explain <target>` call it by name.",
      },
      {
        title: "Checked before it writes",
        body: "A validator mirrors the capture gate, so a candidate that would be discarded gets fixed before `finalize_change`.",
      },
      {
        title: "English inside, your language outside",
        body: "The agent answers in the language you write in and keeps tool names, Record ids and commands verbatim. New Records follow the language your canon already uses.",
      },
      {
        title: "One folder for every agent",
        body: "It follows the open Agent Skills format, so the same directory works in Claude Code, Codex and any agent that reads skills.",
      },
    ],
    skillOpHead: "Operation",
    skillWhenHead: "When the agent uses it",
    skillOps: [
      { op: "preflight", when: "Before it changes, moves or deletes non-trivial code" },
      { op: "explain", when: "When code looks redundant or odd, or you ask why it exists" },
      { op: "capture", when: "When a change is done and tested" },
      { op: "conflicts", when: "When a candidate collides with a pinned rule" },
      { op: "health", when: "When tools are missing or results look degraded" },
      { op: "adopt", when: "When setting Rationale up and seeding the first Records" },
      { op: "maintain", when: "When bindings go stale or `rationale doctor` reports findings" },
    ],
    skillInstallLabel: "Install it from GitHub today",
    skillInstallNote: "`rationale install-agent` installs it for Claude Code and Codex starting with the next release.",
    skillDocs: "Skill guide",

    packetTitle: "What the agent must know, and nothing more.",
    packetBody:
      "A bounded packet compiled for one target and one intent. Governing knowledge is never truncated, and the budget is a ceiling, not a goal.",
    packetPoints: [
      "Constraints and decisions, each with authority and provenance",
      "Why relationships exist, with their live structural state",
      "A bounded subgraph: callers, callees, dependencies, tests",
      "Intent conflicts, risks and honest unknowns",
    ],
    packetFile: "packet.json",
    packetNote: "abridged example",

    roomTitle: "See what every agent was given, as it happens.",
    roomBody:
      "`rationale ui` opens a local, read-only Control Room: the working subgraph in 3D, the memory that explains it, and every session’s activity as it streams.",
    roomFeatures: [
      { title: "Working subgraph", body: "Nodes colored by their role in the change, edges by structural state." },
      { title: "Causal overlay", body: "Rings mark what the canon explains. Select any node to read why." },
      { title: "Live activity", body: "Claude Code, Codex, Cursor and the CLI, streamed over SSE." },
      { title: "Memory browser", body: "Provenance, authority, bindings and pending conflicts." },
    ],
    roomSafety: "Served on 127.0.0.1 only, GET requests only, embedded in the binary.",
    roomDocs: "Control Room docs",

    memoryTitle: "Agents write memory. You keep authority.",
    memoryBody: "Every Record carries where it came from and how far it can move. The capture gate enforces that boundary.",
    memoryTerms: [
      {
        term: "provenance",
        title: "Always attributed",
        body: "An agent’s assertion records its client, session and operation. It is never presented as something a person stated.",
      },
      {
        term: "pinned",
        title: "Rules that hold",
        body: "Pin a rule and every agent can use it. None can replace it.",
      },
      {
        term: "conflicts",
        title: "You decide",
        body: "An agent that tries to supersede a pinned rule writes nothing. You keep the rule or adopt the new one.",
      },
    ],
    memoryTerminalNote: "Human-only commands: they require an interactive terminal and declared authority.",
    commentPin: "# agents can use it, never replace it",
    commentConflicts: "# assertions that collided with a pinned rule",
    commentResolve: "# your call, recorded for audit",

    edgesTitle: "What calls what, and why it has to.",
    edgesBody:
      "A Record can explain a relationship. On every call Rationale re-derives its state from the provider, and it never deletes the explanation.",
    edgesStateHead: "State",
    edgesSampleHead: "Edge",
    edgesMeaningHead: "What it means",
    edgeStates: [
      { state: "observed", body: "The direct relationship exists in the current index." },
      { state: "indirect", body: "No longer direct, but a bounded path still connects both ends." },
      { state: "orphaned", body: "It can’t be located. The explanation may be stale, so it is flagged and never deleted." },
      { state: "unknown", body: "The provider couldn’t verify it. Absence is not proof." },
    ],

    systemsTitle: "Structure tells you where. Git tells you what. Rationale protects why.",
    systemsBody:
      "Agents reach the canon only through the capture gate, and the Control Room only reads. This is the binary’s real wiring, drawn from its own call graph.",
    systemsDocs: "Architecture docs",

    agentsTitle: "Works with the agents you already use.",
    agentsBody:
      "The installer registers one MCP server per agent with its absolute path; `install-agent` writes the protocol into your project. Remove it all with `uninstall-agent`.",
    agentsDocs: "Agents and MCP docs",
    claudeTitle: "The skill and five shortcuts",
    claudeNote: "The agent picks the skill on its own; the shortcuts are for you. `/rationale-conflicts` hands the decision to you.",
    codexTitle: "Ask in plain language",
    codexPrepare: "Prepare this change with Rationale for `<target>` with intent `<intent>`.",
    codexExplain: "Explain `<target>` before changing it.",
    codexCapture: "Capture this change with Rationale.",
    codexHealth: "Check Rationale health.",
    codexNote: "Codex reads the protocol from `AGENTS.md` and calls the MCP tools. With the skill installed, `$rationale` calls it by name.",
    cursorTitle: "An always-on rule",
    cursorBody: "Cursor gets `.cursor/rules/rationale.mdc` in the project and a user-scoped MCP server that works from the Dock.",

    trustTitle: "Local by default. Honest about uncertainty.",
    trustItems: [
      { title: "Canon in Git", body: "`.rationale/` is plain YAML, reviewed in the same pull request as the code." },
      { title: "Traces stay home", body: "Activity keeps identifiers and a one-line intent. Never code, prompts or Record content." },
      { title: "Read-only mission control", body: "127.0.0.1, GET-only, Host-validated, embedded assets." },
      { title: "Provider boundary", body: "Codebase Memory only through its public tools. Unavailable means unknown, never complete." },
    ],
    trustSpec: "5 MCP tools · 6 MCP prompts · 1 skill with 7 operations · 3 supported agents · 5 platforms · no accounts, no telemetry",

    releaseTitle: "1.0 is stable. Here is what that means.",
    releaseBody: "The full loop is implemented, tested and used on Rationale’s own repository.",
    releaseVerified: [
      "366 Rust tests, Clippy, RustSec and schema validation",
      "Clean-checkout and packaged-binary verification",
      "Registration migration for Claude Code, Codex and Cursor",
      "Dogfood: a real change captured and retrieved end to end",
    ],
    releaseOpen: [
      "The `rationale` skill is on `main`; `install-agent` ships it in the next release",
      "Lexical intent-conflict polarity is a noisy hint",
      "Some ADRs are implemented but still proposed",
    ],
    releaseVerifiedLabel: "Verified",
    releaseOpenLabel: "Still open",
    releaseEvidence: "Read the evidence",
    releaseLimits: "Known limits",

    docsTitle: "Everything you need to check the claims.",
    docsCards: [
      { slug: "quickstart", title: "Five-minute quickstart", body: "Install, connect an agent, first governed change." },
      { slug: "skill", title: "The rationale skill", body: "Operations, playbooks, the validator and your language." },
      { slug: "workflow", title: "The loop", body: "Prepare, change, capture, and where a person decides." },
      { slug: "control-room", title: "Control Room", body: "The graph, the memory and live activity." },
      { slug: "concepts", title: "Core concepts", body: "Records, provenance, authority, relationships." },
      { slug: "evidence", title: "Evidence", body: "What 1.0 was verified against, and what’s open." },
    ],

    footerLine: "Your repo. Your process. Your context.",
    footerDocs: "Docs",
    footerGithub: "GitHub",
    footerLicense: "MIT License",
    footerStatus: "v1.0.0 · no remote telemetry",
  },
  es: {
    skip: "Saltar al contenido principal",
    navLoop: "El ciclo",
    navStart: "Empezar",
    navSkill: "Skill",
    navControlRoom: "Control Room",
    navDocs: "Docs",
    githubLabel: "Rationale en GitHub",
    languageLabel: "Read in English",
    themeLabel: "Tema oscuro",
    brandTagline: "Memoria causal para agentes de código",

    heroTitleA: "Tus agentes olvidan el porqué.",
    heroTitleB: "Rationale lo recuerda.",
    heroBody:
      "Antes de que un agente cambie tu código, Rationale le entrega las reglas, decisiones y relaciones que gobiernan ese código. Después del cambio, lo que sigue siendo cierto vuelve a tu repositorio de forma automática, y las reglas que fijas quedan fuera del alcance de cualquier agente.",
    startCta: "Empezar",
    quickstartCta: "Guía rápida",
    heroMeta: "v1.0.0 estable · Git y Codebase Memory · sin cuenta · sin telemetría · 127.0.0.1",

    problemTitle: "Explicaste la regla. El siguiente chat nunca la escuchó.",
    problemMonday: "Lunes, tú",
    problemThursday: "Jueves, un agente nuevo",
    problemQuote: "“Nunca reintentes un pago que ya llegó al banco.”",
    problemCommit: "commit: limpieza",
    problemBody:
      "Funciona en la conversación del lunes. El jueves, un agente nuevo solo ve el código, quita la guarda y lo llama limpieza.",
    problemAnswer:
      "Rationale mantiene la regla junto al código que gobierna y se la entrega a cada agente antes de que toque ese código.",

    loopTitle: "Contexto antes del cambio. Memoria después.",
    loopBody: "No hay cola de aprobación. Los agentes capturan lo que aprenden; tú conservas la autoridad sobre lo que no debe moverse.",
    loopSteps: [
      { tool: "Codebase Memory", code: false, title: "Localizar", body: "Encuentra el símbolo, sus callers y los archivos alrededor." },
      {
        tool: "prepare_change",
        code: true,
        title: "Preparar",
        body: "Reglas gobernantes, relaciones explicadas, la vecindad estructural y el código del target, dentro de un techo de tokens.",
      },
      { tool: "tus tests", code: false, title: "Cambiar", body: "El cambio mínimo coherente con ese contexto." },
      {
        tool: "finalize_change",
        code: true,
        title: "Capturar",
        body: "El conocimiento durable se vuelve Records canónicos en la misma llamada. El ruido y los duplicados se descartan con motivo.",
      },
      {
        tool: "rationale pin",
        code: true,
        title: "Fijar",
        body: "Fija las reglas que ningún agente puede reemplazar. Cualquier intento se vuelve un conflicto que solo tú resuelves.",
      },
    ],

    startTitle: "Funcionando en cinco minutos.",
    startBody:
      "Se instala una vez por máquina y se inicializa una vez por repositorio; después sigues trabajando como ya lo haces. Actualizar o desinstalar nunca toca `.rationale/`.",
    startCompanionA: "Instala",
    startCompanionB: " (recomendado)",
    startInstall: "Instala Rationale en macOS o Linux",
    startInstallNote: "El instalador verifica checksums, usa el canal estable y registra el servidor MCP en los agentes que encuentra.",
    startInit: "Inicializa tu repositorio",
    startInitNote: "Crea el canon en `.rationale/` y escribe el protocolo en `CLAUDE.md`, `AGENTS.md` o la regla de Cursor.",
    startRestart: "Reinicia tu agente y comprueba la conexión",
    startRestartNote: "En Claude Code ejecuta `/rationale-health`. En Codex pide «Comprueba la salud de Rationale».",
    startAsk: "Pide un cambio real",
    startAskQuote: "“Haz que los reintentos de pago se detengan después de tres intentos.”",
    startAskNote: "No necesitas mencionar Rationale. El agente prepara el cambio, lo hace y captura lo que sigue siendo cierto.",
    startWatch: "Míralo en vivo y fija lo que no debe moverse",
    startWindows: "En Windows, el instalador de PowerShell está en la guía rápida.",
    copyLabel: "Copiar",

    skillTitle: "Un skill le enseña a tu agente todo el trabajo.",
    skillBody:
      "El protocolo en `CLAUDE.md` y `AGENTS.md` le dice al agente que prepare y capture. El skill `rationale` le muestra cómo. Un router corto carga solo el playbook de la operación en curso, así que esa profundidad no cuesta nada hasta que una tarea la necesita.",
    skillFeatures: [
      {
        title: "Entra cuando corresponde",
        body: "Claude Code y Codex pueden cargarlo por su cuenta cuando la tarea coincide. `/rationale capture` o `$rationale explain <target>` lo llaman por nombre.",
      },
      {
        title: "Revisa antes de escribir",
        body: "Un validador replica el gate de captura, así que un candidato que se descartaría se corrige antes de `finalize_change`.",
      },
      {
        title: "Instrucciones en inglés, respuestas en tu idioma",
        body: "El agente responde en el idioma en que escribes y mantiene literales los nombres de herramientas, los ids de Records y los comandos. Los Records nuevos siguen el idioma que ya usa tu canon.",
      },
      {
        title: "Una carpeta para cada agente",
        body: "Sigue el formato abierto Agent Skills: el mismo directorio funciona en Claude Code, Codex y cualquier agente que lea skills.",
      },
    ],
    skillOpHead: "Operación",
    skillWhenHead: "Cuándo la usa el agente",
    skillOps: [
      { op: "preflight", when: "Antes de cambiar, mover o borrar código no trivial" },
      { op: "explain", when: "Cuando el código parece redundante o raro, o preguntas por qué existe" },
      { op: "capture", when: "Cuando un cambio está hecho y probado" },
      { op: "conflicts", when: "Cuando un candidato choca con una regla fijada" },
      { op: "health", when: "Cuando faltan herramientas o los resultados se ven degradados" },
      { op: "adopt", when: "Al configurar Rationale y sembrar los primeros Records" },
      { op: "maintain", when: "Cuando los bindings quedan obsoletos o `rationale doctor` reporta hallazgos" },
    ],
    skillInstallLabel: "Instálalo hoy desde GitHub",
    skillInstallNote: "`rationale install-agent` lo instala para Claude Code y Codex a partir de la próxima release.",
    skillDocs: "Guía del skill",

    packetTitle: "Lo que el agente debe saber, y nada más.",
    packetBody:
      "Un packet acotado, compilado para un target y una intención. El conocimiento gobernante nunca se recorta, y el presupuesto es un techo, no una meta.",
    packetPoints: [
      "Constraints y decisiones, cada una con autoridad y procedencia",
      "Por qué existen las relaciones, con su estado estructural vivo",
      "Un subgrafo acotado: callers, callees, dependencias, tests",
      "Conflictos con la intención, riesgos y desconocidos honestos",
    ],
    packetFile: "packet.json",
    packetNote: "ejemplo abreviado",

    roomTitle: "Mira qué recibió cada agente, mientras ocurre.",
    roomBody:
      "`rationale ui` abre un Control Room local y de solo lectura: el subgrafo de trabajo en 3D, la memoria que lo explica y la actividad de cada sesión mientras llega.",
    roomFeatures: [
      { title: "Subgrafo de trabajo", body: "Nodos coloreados por su rol en el cambio, aristas por estado estructural." },
      { title: "Overlay causal", body: "Los anillos marcan lo que explica el canon. Selecciona un nodo para leer el porqué." },
      { title: "Actividad en vivo", body: "Claude Code, Codex, Cursor y la CLI, por SSE." },
      { title: "Navegador de memoria", body: "Procedencia, autoridad, bindings y conflictos pendientes." },
    ],
    roomSafety: "Solo en 127.0.0.1, solo peticiones GET, embebido en el binario.",
    roomDocs: "Docs del Control Room",

    memoryTitle: "Los agentes escriben memoria. Tú conservas la autoridad.",
    memoryBody: "Cada Record lleva de dónde viene y cuánto se puede mover. El gate de captura hace cumplir esa frontera.",
    memoryTerms: [
      {
        term: "provenance",
        title: "Siempre atribuido",
        body: "La afirmación de un agente registra su cliente, sesión y operación. Nunca se presenta como algo que declaró una persona.",
      },
      {
        term: "pinned",
        title: "Reglas que se sostienen",
        body: "Fija una regla y todos los agentes pueden usarla. Ninguno puede reemplazarla.",
      },
      {
        term: "conflicts",
        title: "Tú decides",
        body: "Un agente que intenta reemplazar una regla fijada no escribe nada. Conservas la regla o adoptas la nueva.",
      },
    ],
    memoryTerminalNote: "Comandos solo humanos: exigen una terminal interactiva y autoridad declarada.",
    commentPin: "# los agentes la usan, nunca la reemplazan",
    commentConflicts: "# afirmaciones que chocaron con una regla fijada",
    commentResolve: "# tu decisión, registrada para auditoría",

    edgesTitle: "Qué llama a qué, y por qué tiene que hacerlo.",
    edgesBody:
      "Un Record puede explicar una relación. En cada llamada Rationale vuelve a derivar su estado desde el proveedor, y nunca borra la explicación.",
    edgesStateHead: "Estado",
    edgesSampleHead: "Arista",
    edgesMeaningHead: "Qué significa",
    edgeStates: [
      { state: "observed", body: "La relación directa existe en el índice actual." },
      { state: "indirect", body: "Ya no es directa, pero un camino acotado sigue conectando los extremos." },
      { state: "orphaned", body: "No se puede localizar. La explicación puede estar obsoleta: se señala y nunca se borra." },
      { state: "unknown", body: "El proveedor no pudo verificarla. La ausencia no es prueba." },
    ],

    systemsTitle: "La estructura dice dónde. Git dice qué. Rationale protege por qué.",
    systemsBody:
      "Los agentes solo llegan al canon a través del gate de captura, y el Control Room solo lee. Es el cableado real del binario, dibujado desde su propio grafo de llamadas.",
    systemsDocs: "Docs de arquitectura",

    agentsTitle: "Funciona con los agentes que ya usas.",
    agentsBody:
      "El instalador registra un servidor MCP por agente con su ruta absoluta; `install-agent` escribe el protocolo en tu proyecto. Retíralo todo con `uninstall-agent`.",
    agentsDocs: "Agentes y MCP en los docs",
    claudeTitle: "El skill y cinco atajos",
    claudeNote: "El agente elige el skill por su cuenta; los atajos son para ti. `/rationale-conflicts` te deja la decisión a ti.",
    codexTitle: "Pídelo por escrito",
    codexPrepare: "Prepara este cambio con Rationale para `<target>` con intención `<intent>`.",
    codexExplain: "Explícame `<target>` antes de modificarlo.",
    codexCapture: "Captura este cambio con Rationale.",
    codexHealth: "Comprueba la salud de Rationale.",
    codexNote: "Codex lee el protocolo en `AGENTS.md` y llama las herramientas MCP. Con el skill instalado, `$rationale` lo llama por nombre.",
    cursorTitle: "Una regla siempre activa",
    cursorBody: "Cursor recibe `.cursor/rules/rationale.mdc` en el proyecto y un servidor MCP por usuario que funciona desde el Dock.",

    trustTitle: "Local por defecto. Honesto sobre la incertidumbre.",
    trustItems: [
      { title: "Canon en Git", body: "`.rationale/` es YAML plano, revisado en el mismo pull request que el código." },
      { title: "Las trazas no salen", body: "La actividad guarda identificadores y una intención de una línea. Nunca código, prompts ni contenido de Records." },
      { title: "Centro de control de solo lectura", body: "127.0.0.1, solo GET, Host validado, assets embebidos." },
      { title: "Frontera con el proveedor", body: "Codebase Memory solo por sus herramientas públicas. No disponible significa desconocido, nunca completo." },
    ],
    trustSpec: "5 herramientas MCP · 6 prompts MCP · 1 skill con 7 operaciones · 3 agentes soportados · 5 plataformas · sin cuentas ni telemetría",

    releaseTitle: "1.0 es estable. Esto es lo que significa.",
    releaseBody: "El ciclo completo está implementado, probado y en uso sobre el propio repositorio de Rationale.",
    releaseVerified: [
      "366 tests de Rust, Clippy, RustSec y validación de schemas",
      "Verificación desde checkout limpio y del binario empaquetado",
      "Migración del registro en Claude Code, Codex y Cursor",
      "Dogfood: un cambio real capturado y recuperado de extremo a extremo",
    ],
    releaseOpen: [
      "El skill `rationale` está en `main`; `install-agent` lo incluye en la próxima release",
      "La polaridad léxica de los conflictos es una pista ruidosa",
      "Algunos ADRs están implementados pero siguen propuestos",
    ],
    releaseVerifiedLabel: "Verificado",
    releaseOpenLabel: "Sigue abierto",
    releaseEvidence: "Leer la evidencia",
    releaseLimits: "Límites conocidos",

    docsTitle: "Todo lo necesario para verificar lo que prometemos.",
    docsCards: [
      { slug: "quickstart", title: "Guía rápida de cinco minutos", body: "Instala, conecta un agente, primer cambio gobernado." },
      { slug: "skill", title: "El skill rationale", body: "Operaciones, playbooks, el validador y tu idioma." },
      { slug: "workflow", title: "El ciclo", body: "Preparar, cambiar, capturar, y dónde decide una persona." },
      { slug: "control-room", title: "Control Room", body: "El grafo, la memoria y la actividad en vivo." },
      { slug: "concepts", title: "Conceptos clave", body: "Records, procedencia, autoridad, relaciones." },
      { slug: "evidence", title: "Evidencia", body: "Contra qué se verificó 1.0 y qué sigue abierto." },
    ],

    footerLine: "Tu repo. Tu proceso. Tu contexto.",
    footerDocs: "Docs",
    footerGithub: "GitHub",
    footerLicense: "Licencia MIT",
    footerStatus: "v1.0.0 · sin telemetría remota",
  },
} as const;
