import { FolderOpen } from "lucide-react";
import { Card, EmptyState, Text } from "qing-steel";
import { useApp } from "../app/state";
import { PageHeading, ProjectLink } from "../app/common";

export function ExamplePage() {
  const { active } = useApp();
  return (
    <>
      <PageHeading title="工作台" />
      {active ? (
        <Card title="当前项目">
          <dl className="detail-list">
            <dt>名称</dt>
            <dd>{active.name}</dd>
            <dt>编号</dt>
            <dd>
              <Text variant="data">{active.id}</Text>
            </dd>
            <dt>描述</dt>
            <dd>{active.description || "未填写"}</dd>
          </dl>
          <ProjectLink />
        </Card>
      ) : (
        <Card>
          <EmptyState
            icon={<FolderOpen />}
            title="尚未打开项目"
            description="选择或新建一个项目后，可在这里查看项目内容。"
            action={<ProjectLink />}
          />
        </Card>
      )}
    </>
  );
}
