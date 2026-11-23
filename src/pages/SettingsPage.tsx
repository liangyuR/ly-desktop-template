import { useEffect, useState } from "react";
import { Button, Card, Checkbox, NumberField } from "qing-steel";
import { Monitor, Moon, Sun } from "lucide-react";
import { Failure, PageHeading } from "../app/common";
import { useApp } from "../app/state";
import { call, desktop, errorText, type Mcp, type Theme } from "../desktop";

export function SettingsPage() {
  const { theme, setTheme, settings } = useApp();
  const [themeError, setThemeError] = useState("");
  const [savingTheme, setSavingTheme] = useState(false);
  async function choose(mode: Theme) {
    setSavingTheme(true);
    setThemeError("");
    try {
      await setTheme(mode);
    } catch (e) {
      setThemeError(errorText(e));
    } finally {
      setSavingTheme(false);
    }
  }
  return (
    <>
      <PageHeading title="设置" />
      <Card title="外观">
        <div className="theme-options" role="group" aria-label="主题">
          {(
            [
              { mode: "system", label: "跟随系统", icon: Monitor },
              { mode: "light", label: "浅色", icon: Sun },
              { mode: "dark", label: "深色", icon: Moon },
            ] as const
          ).map(({ mode, label, icon: Icon }) => (
            <button
              key={mode}
              className="theme-choice"
              aria-pressed={theme === mode}
              disabled={savingTheme}
              onClick={() => void choose(mode)}
            >
              <Icon size={20} />
              {label}
            </button>
          ))}
        </div>
        <Failure text={themeError || settings.error} retry={settings.refresh} />
      </Card>
      <McpSettings />
    </>
  );
}

function McpSettings() {
  const [status, setStatus] = useState<Mcp | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [port, setPort] = useState<number | "">("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    if (!desktop) return;
    let active = true;
    call<Mcp>("mcp_status")
      .then((s) => {
        if (active) {
          setStatus(s);
          setEnabled(s.enabled);
          setPort(s.port);
          setError("");
        }
      })
      .catch((e) => {
        if (active) setError(errorText(e));
      });
    return () => {
      active = false;
    };
  }, [retry]);
  async function save() {
    setSaving(true);
    setError("");
    try {
      const next = await call<Mcp>("mcp_configure", { enabled, port });
      setStatus(next);
      setEnabled(next.enabled);
      setPort(next.port);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  }
  return (
    <Card
      title="本机 MCP"
      description="启用后，本机客户端可连接此应用注册的工具。"
      actions={
        <span className={"state-pill" + (status?.enabled ? " positive" : "")}>
          {!desktop
            ? "桌面端可用"
            : !status
              ? "未读取"
              : status.enabled
                ? "运行中"
                : "已关闭"}
        </span>
      }
    >
      <div className="form-stack compact">
        <Checkbox
          label="启用本机服务"
          checked={enabled}
          disabled={!status || saving}
          onCheckedChange={setEnabled}
        />
        <NumberField
          label="监听端口"
          min={1}
          max={65535}
          value={port}
          onValueChange={setPort}
          disabled={!status || saving}
        />
        {status?.url && <p className="mono selectable">{status.url}</p>}
        <Failure
          text={error}
          retry={!status ? () => setRetry((n) => n + 1) : undefined}
        />
        <div>
          <Button
            variant="tonal"
            onClick={() => void save()}
            disabled={
              !status ||
              saving ||
              !Number.isInteger(port) ||
              Number(port) < 1 ||
              Number(port) > 65535 ||
              (enabled === status.enabled && port === status.port)
            }
          >
            应用设置
          </Button>
        </div>
      </div>
    </Card>
  );
}
