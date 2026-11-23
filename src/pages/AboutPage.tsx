import { useState } from "react";
import { Button, Card, Dialog, Stack, Text } from "qing-steel";
import { FileText } from "lucide-react";
import { Failure, PageHeading } from "../app/common";
import { useApp } from "../app/state";
import { call, desktop, errorText } from "../desktop";

export function AboutPage() {
  const { info, update } = useApp();
  const [error, setError] = useState("");
  const [checking, setChecking] = useState(false);
  const [confirmVersion, setConfirmVersion] = useState<string>();
  const [installing, setInstalling] = useState(false);
  const check = async () => {
    setChecking(true);
    await update.refresh();
    setChecking(false);
  };
  const install = async () => {
    setInstalling(true);
    setError("");
    try {
      await call("update_install", { expectedVersion: confirmVersion });
    } catch (e) {
      setError(errorText(e));
    } finally {
      setInstalling(false);
      setConfirmVersion(undefined);
    }
  };
  return (
    <>
      <PageHeading title="关于" />
      <Card title={info.value?.name ?? "桌面应用"}>
        <Failure text={info.error} retry={info.refresh} />
        <dl className="detail-list">
          <dt>版本</dt>
          <dd>{info.value?.version ?? "—"}</dd>
          <dt>源码修订</dt>
          <dd>
            <Text variant="data">{info.value?.commit ?? "—"}</Text>
          </dd>
          <dt>构建时间</dt>
          <dd>{info.value?.builtAt ?? "—"}</dd>
          <dt>数据目录</dt>
          <dd>
            <Text variant="data">{info.value?.dataDir ?? "—"}</Text>
          </dd>
          <dt>日志目录</dt>
          <dd>
            <Text variant="data">{info.value?.logDir ?? "—"}</Text>
          </dd>
        </dl>
        <Button
          variant="outlined"
          leadingIcon={<FileText className="qs-ic" />}
          disabled={!desktop}
          onClick={() => {
            setError("");
            void call<void>("open_current_log").catch((e) =>
              setError(errorText(e)),
            );
          }}
        >
          打开当前日志
        </Button>
      </Card>
      <Card title="在线更新">
        <Text as="p">
          {update.value?.status === "available"
            ? "发现新版本 " + update.value.version
            : update.value?.status === "current"
              ? "当前已是最新版本"
              : update.value?.status === "unconfigured"
                ? "此应用尚未配置更新源。"
                : desktop
                  ? (update.error ? "未取得可用更新。" : "等待更新检查结果。")
                  : "在桌面应用中检查更新。"}
        </Text>
        {update.value?.notes && (
          <Text as="p" className="release-notes">{update.value.notes}</Text>
        )}
        <Failure text={update.error} />
        <Stack>
          <Button
            disabled={!desktop || checking || installing}
            onClick={() => void check()}
          >
            {checking ? "正在检查…" : "检查更新"}
          </Button>
          {update.value?.status === "available" && (
            <Button
              variant="filled"
              onClick={() => setConfirmVersion(update.value?.version)}
              disabled={installing || checking}
            >
              安装更新
            </Button>
          )}
        </Stack>
      </Card>
      <Failure text={error} />
      {confirmVersion && confirmVersion === update.value?.version && (
        <Dialog
          open
          title="安装更新并重启？"
          closeDisabled={installing}
          onOpenChange={(open) => {
            if (!open && !installing) setConfirmVersion(undefined);
          }}
          footer={
            <>
              <Button disabled={installing} onClick={() => setConfirmVersion(undefined)}>
                稍后
              </Button>
              <Button
                variant="filled"
                disabled={installing}
                onClick={() => void install()}
              >
                {installing ? "安装中…" : "下载、安装并重启"}
              </Button>
            </>
          }
        >
          请先保存正在编辑的内容。应用将下载并验证 {confirmVersion}{" "}
          更新包，安装完成后重启。
        </Dialog>
      )}
    </>
  );
}
