function boot() {
  const root = document.documentElement;
  const themeToggle = document.querySelector("[data-theme-toggle]");
  const copyButtons = document.querySelectorAll("[data-copy-value]");
  const githubStars = document.querySelector("[data-github-stars]");
  const copy = root.lang === "es"
    ? {
        copied: "Copiado",
        copyFailed: "Selecciona y copia",
        copyCommand: "Copiar comando",
      }
    : {
        copied: "Copied",
        copyFailed: "Select and copy",
        copyCommand: "Copy command",
      };

  const savedTheme = window.localStorage.getItem("rationale-theme");
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;

  function setTheme(theme) {
    const selected = theme === "dark" ? "dark" : "light";
    root.dataset.theme = selected;
    window.localStorage.setItem("rationale-theme", selected);
    if (themeToggle) {
      const label = selected === "dark"
        ? (root.lang === "es" ? "Modo claro" : "Light mode")
        : (root.lang === "es" ? "Modo oscuro" : "Dark mode");
      themeToggle.setAttribute("aria-label", label);
      themeToggle.setAttribute("aria-pressed", String(selected === "dark"));
      themeToggle.querySelector("[data-theme-icon]").textContent = selected === "dark" ? "☼" : "◐";
    }
  }

  setTheme(savedTheme || (prefersDark ? "dark" : "light"));
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

  async function loadGithubStars() {
    if (!githubStars) return;
    const cacheKey = "rationale-github-stars";
    const cacheTtlMs = 60 * 60 * 1000;
    try {
      const cached = JSON.parse(window.localStorage.getItem(cacheKey) || "null");
      if (cached && Date.now() - cached.fetchedAt < cacheTtlMs && Number.isFinite(cached.count)) {
        githubStars.textContent = String(cached.count);
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
      githubStars.textContent = String(count);
      try {
        window.localStorage.setItem(cacheKey, JSON.stringify({ count, fetchedAt: Date.now() }));
      } catch {
        // The live value is still useful when browser storage is unavailable.
      }
    } catch {
      // Keep the rendered fallback and the GitHub link when the API is down.
    }
  }

  loadGithubStars();
}

export { boot };
