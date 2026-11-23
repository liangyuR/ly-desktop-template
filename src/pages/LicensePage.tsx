import { useState } from "react";
import {
  Button,
  Card,
  StatusBadge,
  Text,
  TextField,
  type StatusBadgeTone,
} from "qing-steel";
import { PageHeading, Failure } from "../app/common";
import { useApp } from "../app/state";
import { call, desktop, errorText, type License } from "../desktop";

const licenseStates = {
  unconfigured: { label: "待配置", tone: "neutral" },
  unactivated: { label: "未激活", tone: "off" },
  valid: { label: "有效", tone: "ok" },
  expiring: { label: "即将到期", tone: "warn" },
  expired: { label: "已到期", tone: "ng" },
  invalid: { label: "证书无效", tone: "ng" },
  recovery: { label: "需要处理", tone: "warn" },
} satisfies Record<License["state"], { label: string; tone: StatusBadgeTone }>;

export function LicensePage() {
  const { license } = useApp();
  const [id, setId] = useState("");
  const [secret, setSecret] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const value = license.value;
  const status = value ? licenseStates[value.state] : undefined;
  const act = async (activate: boolean) => {
    setError("");
    setBusy(true);
    try {
      const next = activate
        ? await call<License>("license_activate", {
            licenseId: id.trim(),
            activationSecret: secret.trim(),
          })
        : await call<License>("license_sync");
      license.accept(next);
      if (activate) setSecret("");
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <PageHeading title="软件授权" />
      <Card
        title="本机授权"
        actions={
          <StatusBadge tone={status?.tone ?? "neutral"}>
            {status?.label ?? (desktop ? "未读取" : "桌面端可用")}
          </StatusBadge>
        }
      >
        <Failure text={license.error || error} retry={license.refresh} />
        {value && (
          <>
            <Text as="p">{value.message}</Text>
            <dl className="detail-list">
              {value.licenseId && (
                <>
                  <dt>授权编号</dt>
                  <dd>
                    <Text variant="data">{value.licenseId}</Text>
                  </dd>
                </>
              )}
              {value.expiresAt && (
                <>
                  <dt>到期时间</dt>
                  <dd>{new Date(value.expiresAt).toLocaleString()}</dd>
                </>
              )}
            </dl>
          </>
        )}
        {value?.configured && value.licenseId && (
          <Button
            variant="tonal"
            disabled={busy}
            onClick={() => void act(false)}
          >
            更新授权
          </Button>
        )}
        {value?.configured &&
          (!value.licenseId ||
            value.state === "invalid" ||
            value.state === "recovery") && (
            <form
              className="form-stack compact"
              onSubmit={(e) => {
                e.preventDefault();
                void act(true);
              }}
            >
              <TextField
                label="授权编号"
                value={id}
                required
                onChange={(e) => setId(e.target.value)}
              />
              <TextField
                label="激活凭据"
                type="password"
                autoComplete="off"
                required
                value={secret}
                onChange={(e) => setSecret(e.target.value)}
              />
              <div>
                <Button
                  type="submit"
                  variant="filled"
                  disabled={busy || !id.trim() || !secret.trim()}
                >
                  激活授权
                </Button>
              </div>
            </form>
          )}
      </Card>
    </>
  );
}
