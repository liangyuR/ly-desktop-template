import { useId, useState, type FormEvent } from "react";
import {
  Button,
  Card,
  Chip,
  Dialog,
  EmptyState,
  IconButton,
  List,
  ListItem,
  Stack,
  Text,
  TextField,
  useSnackbar,
} from "qing-steel";
import {
  Copy,
  Download,
  Folder,
  FolderOpen,
  Pencil,
  Plus,
  Trash2,
  Upload,
} from "lucide-react";
import { useApp } from "../app/state";
import { Failure, PageHeading } from "../app/common";
import { call, desktop, errorText, type Project } from "../desktop";

type Edit =
  { kind: "create" } | { kind: "rename" | "copy" | "delete"; project: Project };

function ProjectDialog({ edit, close }: { edit: Edit; close: () => void }) {
  const { changeProjects, projectBusy } = useApp();
  const formId = useId();
  const [name, setName] = useState(
    edit.kind === "create"
      ? ""
      : edit.project.name + (edit.kind === "copy" ? " 副本" : ""),
  );
  const [description, setDescription] = useState(
    edit.kind === "create" ? "" : edit.project.description,
  );
  const [error, setError] = useState("");
  const title = {
    create: "新建项目",
    rename: "编辑项目",
    copy: "复制项目",
    delete: "删除项目",
  }[edit.kind];
  async function submit(event: FormEvent) {
    event.preventDefault();
    setError("");
    try {
      switch (edit.kind) {
        case "create":
          await changeProjects("projects_create", {
            name: name.trim(),
            description: description.trim(),
          });
          break;
        case "rename":
          await changeProjects("projects_rename", {
            id: edit.project.id,
            name: name.trim(),
            description: description.trim(),
          });
          break;
        case "copy":
          await changeProjects("projects_copy", {
            id: edit.project.id,
            name: name.trim(),
          });
          break;
        case "delete":
          await changeProjects("projects_delete", { id: edit.project.id });
          break;
      }
      close();
    } catch (cause) {
      setError(errorText(cause));
    }
  }
  return (
    <Dialog
      open
      size="sm"
      title={title}
      closeDisabled={projectBusy}
      initialFocus={edit.kind === "delete" ? undefined : "first-field"}
      onOpenChange={(open) => {
        if (!open && !projectBusy) close();
      }}
      footer={
        <>
          <Button onClick={close} disabled={projectBusy}>
            取消
          </Button>
          <Button
            type="submit"
            form={formId}
            variant="filled"
            danger={edit.kind === "delete"}
            disabled={projectBusy || (edit.kind !== "delete" && !name.trim())}
          >
            {edit.kind === "delete" ? "确认删除" : title}
          </Button>
        </>
      }
    >
      <form id={formId} className="form-stack" onSubmit={submit}>
        {edit.kind === "delete" ? (
          <Text as="p">
            删除“{edit.project.name}”及其本机数据？此操作无法撤销。
          </Text>
        ) : (
          <>
            <TextField
              label="项目名称"
              required
              autoComplete="off"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            {edit.kind !== "copy" && (
              <TextField
                label="项目描述"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
            )}
          </>
        )}
        <Failure text={error} />
      </form>
    </Dialog>
  );
}

export function ProjectsPage() {
  const { projects, active, changeProjects, projectBusy } = useApp();
  const [edit, setEdit] = useState<Edit | null>(null);
  const [error, setError] = useState("");
  const [exporting, setExporting] = useState(false);
  const { show } = useSnackbar();
  const busy = projectBusy || exporting;
  const perform = async (operation: () => Promise<unknown>) => {
    setError("");
    try {
      await operation();
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  const exportProject = async (id: string) => {
    setExporting(true);
    try {
      const path = await call<string | null>("projects_export", { id });
      if (path) show({ tone: "ok", text: "已导出至 " + path });
    } finally {
      setExporting(false);
    }
  };
  const list = projects.value?.projects ?? [];
  return (
    <>
      <PageHeading title="项目">
        <Button
          variant="outlined"
          disabled={!desktop || busy}
          onClick={() => void perform(() => changeProjects("projects_import"))}
          leadingIcon={<Upload className="qs-ic" />}
        >
          导入项目
        </Button>
        <Button
          variant="filled"
          disabled={!desktop || busy}
          onClick={() => setEdit({ kind: "create" })}
          leadingIcon={<Plus className="qs-ic" />}
        >
          新建项目
        </Button>
      </PageHeading>
      <Failure text={error || projects.error} retry={projects.refresh} />
      <Card
        title="本机项目"
        actions={
          <Text variant="label" muted>
            {list.length} 个项目
          </Text>
        }
      >
        {!desktop ? (
          <EmptyState
            icon={<FolderOpen />}
            title="桌面预览"
            description="在桌面应用中创建、打开或导入项目。"
          />
        ) : !projects.value ? (
          <Text as="p" role="status">
            {projects.error ? "项目列表不可用" : "正在读取项目…"}
          </Text>
        ) : list.length === 0 ? (
          <EmptyState
            icon={<FolderOpen />}
            title="还没有项目"
            description="新建项目，或从现有项目包导入。"
          />
        ) : (
          <List className="project-list">
            {list.map((project) => {
              const selected = active?.id === project.id;
              return (
                <ListItem
                  key={project.id}
                  selected={selected}
                  title={
                    <>
                      <strong>{project.name}</strong>
                      {selected && (
                        <>
                          {" "}
                          <Chip size="sm">当前项目</Chip>
                        </>
                      )}
                    </>
                  }
                  description={
                    <>
                      {project.description && (
                        <>
                          {project.description}
                          <br />
                        </>
                      )}
                      <Text variant="data" muted>
                        {project.id}
                      </Text>
                    </>
                  }
                  leading={<Folder className="qs-ic" />}
                  trailing={
                    <Stack justify="flex-end">
                      {!selected && (
                        <Button
                          size="sm"
                          disabled={busy}
                          onClick={() =>
                            void perform(() =>
                              changeProjects("projects_switch", { id: project.id }),
                            )
                          }
                        >
                          切换
                        </Button>
                      )}
                      {(
                        [
                          { kind: "rename", icon: Pencil, label: "编辑" },
                          { kind: "copy", icon: Copy, label: "复制" },
                          { kind: "export", icon: Download, label: "导出" },
                          { kind: "delete", icon: Trash2, label: "删除" },
                        ] as const
                      ).map(({ kind, icon: Icon, label }) => (
                        <IconButton
                          key={kind}
                          size="sm"
                          title={label}
                          aria-label={label + project.name}
                          disabled={busy}
                          onClick={() => {
                            if (kind === "export")
                              void perform(() => exportProject(project.id));
                            else setEdit({ kind, project });
                          }}
                        >
                          <Icon className="qs-ic" />
                        </IconButton>
                      ))}
                    </Stack>
                  }
                />
              );
            })}
          </List>
        )}
      </Card>
      {edit && <ProjectDialog edit={edit} close={() => setEdit(null)} />}
    </>
  );
}
