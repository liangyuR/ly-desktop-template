import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Checkbox,
  NumberField,
  SegmentedControl,
  Stack,
  StatusBadge,
  Text,
} from "qing-steel";
import { Monitor, Moon, Sun } from "lucide-react";
import { Failure, PageHeading } from "../app/common";
import { useApp } from "../app/state";
import { call, desktop, errorText, type Mcp, type Theme } from "../desktop";

const themeOptions = (
  [
    { value: "system", label: "跟随系统", icon: Monitor },
    { value: "light", label: "浅色", icon: Sun },
    { value: "dark", label: "深色", icon: Moon },
  ] as const
).map(({ value, label, icon: Icon }) => ({
  value,
  label: (
    <>
      <Icon className="qs-ic" />
      {label}
    </>
  ),
}));

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
        <Stack direction="column" align="stretch" wrap={false}>
          <SegmentedControl
            label="主题"
            value={theme}
            options={themeOptions}
            disabled={savingTheme}
            onValueChange={(value) => {
              const option = themeOptions.find((option) => option.value === value);
              if (option) void choose(option.value);
            }}
          />
          <Failure text={themeError || settings.error} retry={settings.refresh} />
        </Stack>
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
        <StatusBadge tone={!status ? "neutral" : status.enabled ? "ok" : "off"}>
          {!desktop
            ? "桌面端可用"
            : !status
              ? "未读取"
              : status.enabled
                ? "运行中"
                : "已关闭"}
        </StatusBadge>
      }
    >
      <Stack className="compact" direction="column" align="stretch" wrap={false}>
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
        {status?.url && (
          <Text as="p" variant="data" className="selectable">
            {status.url}
          </Text>
        )}
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
      </Stack>
    </Card>
  );
}
