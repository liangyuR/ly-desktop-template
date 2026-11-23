import { useEffect, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { AppBar, IconButton, Stack, useSnackbar } from "qing-steel";
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
        <AppBar
          className="titlebar"
          title={
            <Stack>
              {info.value?.name ?? document.title}
              <div data-slot ref={setDetails} />
            </Stack>
          }
          subtitle={info.value?.version}
          navigation={<img className="brand-mark" src="/app-icon.png" alt="" draggable={false} />}
          actions={
            <>
              <div data-slot ref={setActions} />
              {win && (
                <Stack wrap={false}>
                  <IconButton
                    size="sm"
                    aria-label="最小化"
                    title="最小化"
                    onClick={() => run(win.minimize())}
                  >
                    <Minus className="qs-ic" />
                  </IconButton>
                  <IconButton
                    size="sm"
                    aria-label={maximized ? "还原" : "最大化"}
                    title={maximized ? "还原" : "最大化"}
                    onClick={() => run(win.toggleMaximize())}
                  >
                    {maximized ? <Copy className="qs-ic" /> : <Square className="qs-ic" />}
                  </IconButton>
                  <IconButton
                    size="sm"
                    tone="danger"
                    aria-label="关闭窗口"
                    title="关闭窗口"
                    onClick={() => run(win.close())}
                  >
                    <X className="qs-ic" />
                  </IconButton>
                </Stack>
              )}
            </>
          }
          onMouseDown={(e) => {
            if (
              !win ||
              e.button !== 0 ||
              !(e.target instanceof Element) ||
              e.target.closest("[data-slot], button, a")
            )
              return;
            run(e.detail === 2 ? win.toggleMaximize() : win.startDragging());
          }}
        />
        {children}
      </div>
    </TitlebarSlots.Provider>
  );
}
