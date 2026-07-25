#!/usr/bin/env bash
#
# Captura el entorno de desarrollo local para Rationale.
#
# Rationale_Arquitectura_Conceptual_v0.1.md §5.1 exige registrar el entorno
# principal de desarrollo, pero §5 prohíbe explícitamente guardar número de
# serie, hardware UUID, identificadores privados, rutas personales, tokens
# o credenciales. Este script filtra esos campos deliberadamente.
#
# Salida: .rationale-local/environment.json (ignorado por Git).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
OUT_DIR="${REPO_ROOT}/.rationale-local"
OUT_FILE="${OUT_DIR}/environment.json"

mkdir -p "${OUT_DIR}"

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required to build environment.json" >&2
  exit 1
fi

OS_NAME="$(uname -s)"

# --- Campos comunes a cualquier OS ---
# uname -a incluye el hostname de la máquina, que en macOS suele contener el
# nombre de la persona propietaria (ej. "MacBook-Air-de-Nombre.local"). Se
# redacta explícitamente para no filtrar un identificador personal.
UNAME_A_RAW="$(uname -a)"
UNAME_A="$(printf '%s\n' "${UNAME_A_RAW}" | sed -E 's/[A-Za-z0-9.-]+\.local/<redacted-hostname>/')"
UNAME_M="$(uname -m)"
GIT_VERSION="$(git --version 2>/dev/null || echo "not found")"
DISK_FREE="$(df -h / 2>/dev/null | tail -n +2 | tr -s ' ')"

MODEL_NAME=""
CHIP=""
CORES=""
MEMORY_GB=""
OS_PRODUCT_NAME=""
OS_PRODUCT_VERSION=""
OS_BUILD_VERSION=""
CLANG_VERSION=""
XCODE_SELECT_PATH=""

if [ "${OS_NAME}" = "Darwin" ]; then
  # Extraer únicamente campos no identificadores de una máquina concreta.
  # Excluidos deliberadamente: Serial Number, Hardware UUID, Provisioning UDID,
  # Activation Lock Status, Model Number.
  HW_INFO="$(system_profiler SPHardwareDataType 2>/dev/null || true)"
  MODEL_NAME="$(printf '%s\n' "${HW_INFO}" | awk -F': ' '/Model Name:/ {print $2; exit}')"
  CHIP="$(printf '%s\n' "${HW_INFO}" | awk -F': ' '/Chip:/ {print $2; exit}')"
  CORES="$(printf '%s\n' "${HW_INFO}" | awk -F': ' '/Total Number of Cores:/ {print $2; exit}')"

  MEM_BYTES="$(sysctl -n hw.memsize 2>/dev/null || echo 0)"
  MEMORY_GB="$(awk -v b="${MEM_BYTES}" 'BEGIN { printf "%.0f", b/1024/1024/1024 }')"
  NCPU="$(sysctl -n hw.ncpu 2>/dev/null || echo "")"
  CORES="${CORES:-${NCPU}}"

  SW_VERS="$(sw_vers 2>/dev/null || true)"
  OS_PRODUCT_NAME="$(printf '%s\n' "${SW_VERS}" | awk -F: '/ProductName:/ {print $2}' | tr -d '\t' | xargs)"
  OS_PRODUCT_VERSION="$(printf '%s\n' "${SW_VERS}" | awk -F: '/ProductVersion:/ {print $2}' | tr -d '\t' | xargs)"
  OS_BUILD_VERSION="$(printf '%s\n' "${SW_VERS}" | awk -F: '/BuildVersion:/ {print $2}' | tr -d '\t' | xargs)"

  CLANG_VERSION="$(clang --version 2>/dev/null | head -1 || echo "not found")"
  XCODE_SELECT_PATH="$(xcode-select -p 2>/dev/null || echo "not found")"
else
  MEM_BYTES="$(sysctl -n hw.memsize 2>/dev/null || echo 0)"
  CORES="$(nproc 2>/dev/null || echo "")"
  MEMORY_GB="unknown"
fi

jq -n \
  --arg captured_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg os_name "${OS_NAME}" \
  --arg os_product_name "${OS_PRODUCT_NAME}" \
  --arg os_product_version "${OS_PRODUCT_VERSION}" \
  --arg os_build_version "${OS_BUILD_VERSION}" \
  --arg uname_a "${UNAME_A}" \
  --arg uname_m "${UNAME_M}" \
  --arg model_name "${MODEL_NAME}" \
  --arg chip "${CHIP}" \
  --arg cores "${CORES}" \
  --arg memory_gb "${MEMORY_GB}" \
  --arg git_version "${GIT_VERSION}" \
  --arg clang_version "${CLANG_VERSION}" \
  --arg xcode_select_path "${XCODE_SELECT_PATH}" \
  --arg disk_free "${DISK_FREE}" \
  '{
    captured_at: $captured_at,
    note: "Serial number, hardware UUID, provisioning UDID and personal paths are deliberately excluded (Arquitectura_Conceptual_v0.1 §5).",
    os: {
      kernel: $os_name,
      product_name: $os_product_name,
      product_version: $os_product_version,
      build_version: $os_build_version,
      uname_a: $uname_a,
      arch: $uname_m
    },
    hardware: {
      model_name: $model_name,
      chip: $chip,
      cores: $cores,
      memory_gb: $memory_gb
    },
    toolchain: {
      git_version: $git_version,
      clang_version: $clang_version,
      xcode_select_path: $xcode_select_path
    },
    disk_free_root: $disk_free
  }' > "${OUT_FILE}"

echo "Environment captured at: ${OUT_FILE}"
cat "${OUT_FILE}"
