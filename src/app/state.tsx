import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { SnackbarProvider, ThemeProvider } from "qing-steel";
import {
  call,
  desktop,
  errorText,
  type AppInfo,
  type License,
  type Projects,
  type Theme,
  type Update,
} from "../desktop";

function useDesktopResource<T>(command: string) {
  const [value, setValue] = useState<T | null>(null);
  const [error, setError] = useState("");
  const sequence = useRef(0);
  const accept = useCallback((next: T) => {
    sequence.current++;
    setValue(next);
    setError("");
  }, []);
  const refresh = useCallback(async () => {
    if (!desktop) return;
    const current = ++sequence.current;
    try {
      const result = await call<T>(command);
      if (sequence.current === current) accept(result);
    } catch (cause) {
      if (sequence.current === current) setError(errorText(cause));
    }
  }, [command, accept]);
  useEffect(() => {
    void refresh();
    return () => {
      sequence.current++;
    };
  }, [refresh]);
  return { value, error, refresh, accept };
}

function useDesktopState() {
  const projects = useDesktopResource<Projects>("projects_list");
  const settings = useDesktopResource<{ theme: Theme }>("get_settings");
  const license = useDesktopResource<License>("license_status");
  const update = useDesktopResource<Update>("update_check");
  const info = useDesktopResource<AppInfo>("app_info");
  const [projectBusy, setProjectBusy] = useState(false);
  const mutating = useRef(false);
  async function changeProjects(
    command: string,
    args?: Record<string, unknown>,
  ) {
    if (mutating.current) throw new Error("请等待当前项目操作完成。");
    mutating.current = true;
    setProjectBusy(true);
    try {
      projects.accept(await call<Projects>(command, args));
    } finally {
      mutating.current = false;
      setProjectBusy(false);
    }
  }
  const theme = settings.value?.theme ?? "system";
  async function setTheme(next: Theme) {
    settings.accept(
      desktop
        ? await call<{ theme: Theme }>("set_theme", { theme: next })
        : { theme: next },
    );
  }
  const refreshLicense = license.refresh;
  const warningAt = license.value?.warningAt;
  const expiresAt = license.value?.expiresAt;
  useEffect(() => {
    if (!desktop) return;
    const refresh = () => {
      if (document.visibilityState !== "hidden") void refreshLicense();
    };
    const interval = window.setInterval(refresh, 30_000);
    const timers = [warningAt, expiresAt]
      .map((time) => (time ? Date.parse(time) - Date.now() : NaN))
      .filter((delay) => delay > 0 && delay < 2_147_483_647)
      .map((delay) => window.setTimeout(refresh, delay + 50));
    window.addEventListener("focus", refresh);
    document.addEventListener("visibilitychange", refresh);
    return () => {
      window.clearInterval(interval);
      timers.forEach((id) => window.clearTimeout(id));
      window.removeEventListener("focus", refresh);
      document.removeEventListener("visibilitychange", refresh);
    };
  }, [refreshLicense, warningAt, expiresAt]);
  return {
    projects,
    settings,
    info,
    license,
    update,
    theme,
    setTheme,
    projectBusy,
    changeProjects,
    active:
      projects.value?.projects.find((p) => p.id === projects.value?.activeId) ??
      null,
  };
}

const Context = createContext<ReturnType<typeof useDesktopState> | null>(null);
export function useApp() {
  const state = useContext(Context);
  if (!state) throw new Error("缺少应用上下文");
  return state;
}
export function AppProvider({ children }: { children: ReactNode }) {
  const state = useDesktopState();
  return (
    <Context.Provider value={state}>
      <ThemeProvider theme={state.theme} className="app-theme">
        <SnackbarProvider>{children}</SnackbarProvider>
      </ThemeProvider>
    </Context.Provider>
  );
}
