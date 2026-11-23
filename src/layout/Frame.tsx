import { useEffect, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { useSnackbar } from "qing-steel";
import { useApp } from "../app/state";
import { desktop, errorText } from "../desktop";
import { TitlebarSlots } from "./TitlebarContent";

export function Frame({ children }: { children: ReactNode }) {
  const { info } = useApp();
  const { show } = useSnackbar();
  const [details, setDetails] = useState<HTMLDivElement | null>(null);
  const [actions, setActions] = useState<HTMLDivElement | null>(null);
  const [maximized, setMaximized] = useState(false);
  const [win] = useState(() => (desktop ? getCurrentWindow() : null));
  useEffect(() => {
    if (!win) return;
    let live = true;
    let seq = 0;
    const refresh = () => {
      const n = ++seq;
      void win
        .isMaximized()
        .then((v) => {
          if (live && n === seq) setMaximized(v);
        })
        .catch((e) => console.error(e));
    };
    refresh();
    let stop: (() => void) | undefined;
    void win
      .onResized(refresh)
      .then((unlisten) => {
        if (live) stop = unlisten;
        else unlisten();
      })
      .catch((e) => console.error(e));
    return () => {
      live = false;
      stop?.();
    };
  }, [win]);
  const run = (p: Promise<unknown>) => {
    void p.catch((e) => show({ tone: "ng", text: errorText(e) }));
  };
  return (
    <TitlebarSlots.Provider value={{ details, actions }}>
      <div className="frame">
        <header className="titlebar">
          <div
            className="title-drag"
            onMouseDown={(e) => {
              if (
                !win ||
                e.button !== 0 ||
                !(e.target instanceof Element) ||
                e.target.closest("[data-slot]")
              )
                return;
              run(e.detail === 2 ? win.toggleMaximize() : win.startDragging());
            }}
          >
            <img className="brand-mark" src="/app-icon.png" alt="" draggable={false} />
            <strong>{info.value?.name ?? document.title}</strong>
            {info.value?.version && (
              <span className="version">{info.value.version}</span>
            )}
            <div data-slot ref={setDetails} />
          </div>
          <div data-slot ref={setActions} />
          {win && (
            <div className="window-buttons">
              <button
                aria-label="最小化"
                title="最小化"
                onClick={() => run(win.minimize())}
              >
                <Minus />
              </button>
              <button
                aria-label={maximized ? "还原" : "最大化"}
                title={maximized ? "还原" : "最大化"}
                onClick={() => run(win.toggleMaximize())}
              >
                {maximized ? <Copy /> : <Square />}
              </button>
              <button
                aria-label="关闭窗口"
                title="关闭窗口"
                onClick={() => run(win.close())}
              >
                <X />
              </button>
            </div>
          )}
        </header>
        {children}
      </div>
    </TitlebarSlots.Provider>
  );
}
