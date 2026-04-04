import { useCallback, useEffect, useRef, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

import { persistLoadConfig } from "../api";
import {
  ZOOM_DEFAULT,
  ZOOM_MAX,
  ZOOM_MIN,
  ZOOM_STEP,
} from "../constants/app";
import { type HelpLanguage } from "../constants/helpContent";
import { resolvePersistStartupEnabled } from "../persistStartupModel";

type ThemeMode = "dark" | "light";

interface UseAppShellStateResult {
  appVersion: string;
  helpLanguage: HelpLanguage;
  persistWanLoading: boolean;
  persistWanOnStartup: boolean;
  theme: ThemeMode;
  themeLensActive: boolean;
  zoomLevel: number;
  handleToggleTheme: () => void;
  handleZoomIn: () => void;
  handleZoomOut: () => void;
  handleZoomReset: () => void;
  setHelpLanguage: (language: HelpLanguage) => void;
  setPersistWanOnStartup: (enabled: boolean) => void;
}

export function useAppShellState(): UseAppShellStateResult {
  const lensTimerRef = useRef<number | null>(null);
  const [appVersion, setAppVersion] = useState("dev");
  const [theme, setTheme] = useState<ThemeMode>(() => {
    const saved = localStorage.getItem("ui-theme");
    return saved === "light" || saved === "dark" ? saved : "dark";
  });
  const [persistWanOnStartup, setPersistWanOnStartupState] = useState(false);
  const [persistWanLoading, setPersistWanLoading] = useState(true);
  const [zoomLevel, setZoomLevel] = useState<number>(() => {
    const saved = localStorage.getItem("app-zoom-level");
    if (saved) {
      const parsed = Number.parseInt(saved, 10);
      if (Number.isFinite(parsed) && parsed >= ZOOM_MIN && parsed <= ZOOM_MAX) {
        return parsed;
      }
    }
    return ZOOM_DEFAULT;
  });
  const [themeLensActive, setThemeLensActive] = useState(false);
  const [helpLanguage, setHelpLanguageState] = useState<HelpLanguage>(() => {
    const saved = localStorage.getItem("help-language");
    return saved === "en" || saved === "vi" ? saved : "vi";
  });

  useEffect(() => {
    let active = true;

    const loadAppVersion = async () => {
      try {
        const version = await getVersion();
        if (active) {
          setAppVersion(version);
        }
      } catch {
        if (active) {
          setAppVersion("dev");
        }
      }
    };

    void loadAppVersion();

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let active = true;
    const savedPreference = localStorage.getItem("wan-persist-on-startup");
    const localPreference =
      savedPreference === "true" ? true : savedPreference === "false" ? false : null;

    const loadPersistStatus = async () => {
      try {
        const persistedConfigResult = await persistLoadConfig();
        const persistedConfigEnabled =
          persistedConfigResult ? persistedConfigResult.enabled : null;

        if (active) {
          setPersistWanOnStartupState(
            resolvePersistStartupEnabled({
              localPreference,
              persistedConfigEnabled,
            }),
          );
        }
      } catch {
        if (active) {
          setPersistWanOnStartupState(
            resolvePersistStartupEnabled({
              localPreference,
              persistedConfigEnabled: null,
            }),
          );
        }
      } finally {
        if (active) {
          setPersistWanLoading(false);
        }
      }
    };

    void loadPersistStatus();

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    const htmlEl = document.documentElement;
    if (zoomLevel === ZOOM_DEFAULT) {
      htmlEl.removeAttribute("data-zoom");
      htmlEl.style.removeProperty("--zoom-level");
    } else {
      htmlEl.setAttribute("data-zoom", String(zoomLevel));
      htmlEl.style.setProperty("--zoom-level", `${zoomLevel}%`);
    }
    localStorage.setItem("app-zoom-level", String(zoomLevel));
  }, [zoomLevel]);

  useEffect(() => {
    localStorage.setItem("ui-theme", theme);
  }, [theme]);

  useEffect(() => {
    localStorage.setItem("wan-persist-on-startup", persistWanOnStartup ? "true" : "false");
  }, [persistWanOnStartup]);

  useEffect(() => {
    localStorage.setItem("help-language", helpLanguage);
  }, [helpLanguage]);

  useEffect(() => {
    return () => {
      if (lensTimerRef.current !== null) {
        window.clearTimeout(lensTimerRef.current);
      }
    };
  }, []);

  const handleZoomIn = useCallback(() => {
    setZoomLevel((previous) => Math.min(ZOOM_MAX, previous + ZOOM_STEP));
  }, []);

  const handleZoomOut = useCallback(() => {
    setZoomLevel((previous) => Math.max(ZOOM_MIN, previous - ZOOM_STEP));
  }, []);

  const handleZoomReset = useCallback(() => {
    setZoomLevel(ZOOM_DEFAULT);
  }, []);

  const handleToggleTheme = useCallback(() => {
    if (lensTimerRef.current !== null) {
      window.clearTimeout(lensTimerRef.current);
    }
    setThemeLensActive(true);
    setTheme((current) => (current === "dark" ? "light" : "dark"));
    lensTimerRef.current = window.setTimeout(() => {
      setThemeLensActive(false);
      lensTimerRef.current = null;
    }, 650);
  }, []);

  const setPersistWanOnStartup = useCallback((enabled: boolean) => {
    setPersistWanOnStartupState(enabled);
  }, []);

  const setHelpLanguage = useCallback((language: HelpLanguage) => {
    setHelpLanguageState(language);
  }, []);

  return {
    appVersion,
    helpLanguage,
    persistWanLoading,
    persistWanOnStartup,
    theme,
    themeLensActive,
    zoomLevel,
    handleToggleTheme,
    handleZoomIn,
    handleZoomOut,
    handleZoomReset,
    setHelpLanguage,
    setPersistWanOnStartup,
  };
}
