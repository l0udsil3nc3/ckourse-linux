import { useState, useEffect, useCallback } from "react";

type ThemeFamily = "default" | "gnome" | "classic";
type ThemeMode = "light" | "dark";

function getFamily(): ThemeFamily {
  if (document.documentElement.classList.contains("gnome")) return "gnome";
  if (document.documentElement.classList.contains("classic")) return "classic";
  return "default";
}

function getMode(): ThemeMode {
  return document.documentElement.classList.contains("light") ? "light" : "dark";
}

function applyTheme(family: ThemeFamily, mode: ThemeMode) {
  // Remove all theme-related classes
  document.documentElement.classList.remove("light", "dark", "gnome", "classic");
  
  // Apply family
  if (family === "gnome") {
    document.documentElement.classList.add("gnome");
  } else if (family === "classic") {
    document.documentElement.classList.add("classic");
  }
  
  // Apply mode
  document.documentElement.classList.add(mode);
}

function getStoredTheme(): { family: ThemeFamily; mode: ThemeMode } {
  if (typeof document === "undefined") {
    const stored = localStorage.getItem("ckourse-theme");
    if (stored) {
      try {
        return JSON.parse(stored);
      } catch {
        // Fallback for old format
        if (stored === "light" || stored === "dark" || stored === "gnome" || stored === "gnome-dark") {
          const family = (stored === "gnome" || stored === "gnome-dark") ? "gnome" : "default";
          const mode = (stored === "light" || stored === "gnome") ? "light" : "dark";
          return { family, mode };
        }
      }
    }
    return { family: "default", mode: "dark" };
  }
  
  return { family: getFamily(), mode: getMode() };
}

export function useTheme() {
  const [{ family, mode }, setThemeState] = useState<{ family: ThemeFamily; mode: ThemeMode }>(() => 
    getStoredTheme()
  );

  const setFamily = useCallback((newFamily: ThemeFamily) => {
    setThemeState(prev => {
      const next = { ...prev, family: newFamily };
      applyTheme(next.family, next.mode);
      localStorage.setItem("ckourse-theme", JSON.stringify(next));
      return next;
    });
  }, []);

  const setMode = useCallback((newMode: ThemeMode) => {
    setThemeState(prev => {
      const next = { ...prev, mode: newMode };
      applyTheme(next.family, next.mode);
      localStorage.setItem("ckourse-theme", JSON.stringify(next));
      return next;
    });
  }, []);

  const toggleTheme = useCallback(() => {
    setMode(mode === "light" ? "dark" : "light");
  }, [mode, setMode]);

  // Observe DOM class changes so we stay in sync with AnimatedThemeToggler
  useEffect(() => {
    const observer = new MutationObserver(() => {
      setThemeState({ family: getFamily(), mode: getMode() });
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  }, []);

  // Apply initial theme from localStorage on mount
  useEffect(() => {
    const stored = getStoredTheme();
    applyTheme(stored.family, stored.mode);
    setThemeState(stored);
  }, []);

  // For backward compatibility, expose combined theme string
  const theme = `${family}${mode === "dark" ? "-dark" : mode === "light" ? "-light" : ""}`.replace("default-", "").replace("-light", "") || (family === "default" ? (mode === "dark" ? "dark" : "light") : (mode === "dark" ? "gnome-dark" : "gnome"));

  return { 
    theme, 
    family, 
    mode,
    setFamily, 
    setMode, 
    toggleTheme 
  } as const;
}
