function boot() {
  const root = document.documentElement;
  const themeToggle = document.querySelector("[data-theme-toggle]");
  const copyButtons = document.querySelectorAll("[data-copy-value]");
  const githubStars = document.querySelector("[data-github-stars]");
  const copy = root.lang === "es"
    ? {
        copied: "Copiado",
        copyFailed: "Selecciona y copia",
        copyCommand: "Copiar",
      }
    : {
        copied: "Copied",
        copyFailed: "Select and copy",
        copyCommand: "Copy",
      };

  let savedTheme = null;
  try {
    savedTheme = window.localStorage.getItem("rationale-theme");
  } catch {
    // Sin almacenamiento, el tema por defecto sigue siendo válido.
  }

  // El botón tiene una etiqueta fija ("Tema oscuro") y aria-pressed dice si
  // está activo; el icono visible lo resuelve el CSS según data-theme.
  function setTheme(theme) {
    const selected = theme === "light" ? "light" : "dark";
    root.dataset.theme = selected;
    try {
      window.localStorage.setItem("rationale-theme", selected);
    } catch {
      // El tema aplica igual aunque no se pueda recordar.
    }
    themeToggle?.setAttribute("aria-pressed", String(selected === "dark"));
  }

  // Oscuro por defecto, como el Control Room; el toggle recuerda la elección.
  setTheme(savedTheme || "dark");
  themeToggle?.addEventListener("click", () => {
    setTheme(root.dataset.theme === "dark" ? "light" : "dark");
  });

  copyButtons.forEach((button) => {
    button.addEventListener("click", async () => {
      const status = button.querySelector("[data-copy-status]");
      try {
        await navigator.clipboard.writeText(button.dataset.copyValue || "");
        status.textContent = copy.copied;
      } catch {
        status.textContent = copy.copyFailed;
      }
      window.setTimeout(() => {
        status.textContent = copy.copyCommand;
      }, 1800);
    });
  });

  // El contador solo aparece con un valor real: nunca se muestra un 0 inventado.
  function showStars(count) {
    githubStars.textContent = String(count);
    githubStars.hidden = false;
  }

  async function loadGithubStars() {
    if (!githubStars) return;
    const cacheKey = "rationale-github-stars";
    const cacheTtlMs = 60 * 60 * 1000;
    try {
      const cached = JSON.parse(window.localStorage.getItem(cacheKey) || "null");
      if (cached && Date.now() - cached.fetchedAt < cacheTtlMs && Number.isFinite(cached.count)) {
        showStars(cached.count);
      }
    } catch {
      // A blocked or malformed localStorage cache should not affect the landing.
    }

    try {
      const response = await fetch("https://api.github.com/repos/Ragosorio/Rationale", {
        headers: { Accept: "application/vnd.github+json" },
      });
      if (!response.ok) return;
      const payload = await response.json();
      if (!Number.isFinite(payload.stargazers_count)) return;
      const count = Number(payload.stargazers_count);
      showStars(count);
      try {
        window.localStorage.setItem(cacheKey, JSON.stringify({ count, fetchedAt: Date.now() }));
      } catch {
        // The live value is still useful when browser storage is unavailable.
      }
    } catch {
      // Keep the GitHub link without a count when the API is down.
    }
  }

  loadGithubStars();
}

export { boot };
