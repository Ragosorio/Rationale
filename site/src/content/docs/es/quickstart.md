---
lang: es
slug: quickstart
title: Quickstart de cinco minutos
description: Instala Rationale, conecta tu agente, haz el primer cambio gobernado y míralo en vivo en el Control Room.
section: Empezar
order: 1
---

## Qué necesitas

Rationale es un único binario local para macOS (Apple Silicon e Intel), Linux
(x86_64 y ARM64) y Windows x86_64. Necesitas Git y una shell. Codebase Memory
es recomendable para el contexto estructural, pero opcional: sin él, Rationale
informa cobertura degradada y sigue funcionando.

## Instalar

Instala primero el compañero estructural:

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash
```

Después instala Rationale:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
rationale --version
```

En Windows, desde PowerShell:

```powershell
$installer = Join-Path $env:TEMP "rationale-installer.ps1"
Invoke-WebRequest https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.ps1 -OutFile $installer
& $installer
rationale.exe --version
```

Los instaladores verifican los checksums SHA-256 y usan el canal `stable`.
Define `RATIONALE_CHANNEL=preview` solo si quieres pre-releases.

## Inicializar un proyecto

Desde el repositorio que quieres proteger:

```bash
rationale init
rationale health
```

`init` crea el canon versionado en `.rationale/` y configura los agentes de
código que detecta. `health` reporta la revisión de Git, el working tree y si
el proveedor estructural está disponible.

Para conectar agentes más tarde, o después de actualizar:

```bash
rationale install-agent --dry-run
rationale install-agent
```

Registra el servidor MCP una vez por usuario (`rationale serve --client <agente>`)
y escribe el protocolo de invocación en `CLAUDE.md`, `AGENTS.md` o la regla de
Cursor. Reinicia el agente después.

## Haz el primer cambio gobernado

Pídele a tu agente un cambio real. Con el protocolo instalado:

1. localiza el código con Codebase Memory;
2. llama a `prepare_change(target, intent)` y lee las reglas, decisiones y
   relaciones explicadas que gobiernan el target;
3. hace el cambio y corre los tests;
4. llama a `finalize_change` solo con el conocimiento que sigue siendo cierto —
   Rationale lo escribe en `.rationale/records/` en esa misma llamada.

En Claude Code puedes guiarlo con `/rationale-preflight <target> <intent>` y
`/rationale-capture`. En Codex, pídelo por escrito: “Prepara este cambio con
Rationale para `<target>` con intención `<intent>`.”

## Míralo en vivo

```bash
rationale ui
```

El Control Room abre en `127.0.0.1:9748`: el subgrafo de trabajo que recibió el
agente, la memoria que lo explica y la actividad de cada sesión mientras
ocurre. Es de solo lectura y nunca sale de tu máquina.

## Conserva las reglas que importan

Cuando una regla no debe ser reemplazada por ningún agente, fíjala:

```bash
rationale pin <record-id>
```

Si más tarde un agente intenta reemplazarla, no se escribe nada: recibes un
conflicto que decides con `rationale conflicts` y `rationale resolve`.
