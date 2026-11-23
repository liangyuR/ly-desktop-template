import { useState, type RefObject } from "react";
import { Link, matchPath, useLocation, useNavigate } from "react-router-dom";
import { FolderKanban, Info, KeyRound, Settings } from "lucide-react";
import { NavigationDrawer, Select, useSnackbar } from "qing-steel";
import { useApp } from "../app/state";
import { modules } from "../app/modules";
import { errorText } from "../desktop";

const items = [
  { path: "/projects", title: "项目", icon: FolderKanban, group: "工作区" },
  ...modules.map((module) => ({ ...module, group: "工作区" })),
  { path: "/settings", title: "设置", icon: Settings, group: "应用" },
  { path: "/license", title: "软件授权", icon: KeyRound, group: "应用" },
  { path: "/about", title: "关于", icon: Info, group: "应用" },
].map(({ path, title, icon: Icon, group }) => ({
  value: path,
  href: path,
  label: title,
  icon: <Icon className="qs-ic" />,
  group,
}));

export function Sidebar({ contentRef }: { contentRef: RefObject<HTMLElement | null> }) {
  const { projects: snapshot, active, changeProjects, projectBusy } = useApp();
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const current = items.find(({ value }) => matchPath(value, pathname))?.value ?? "";
  const { show } = useSnackbar();
  return (
    <NavigationDrawer
      label="应用导航"
      items={items}
      value={current}
      collapsed={collapsed}
      onCollapsedChange={setCollapsed}
      contentRef={contentRef}
      expandLabel="展开侧栏"
      collapseLabel="收起侧栏"
      renderLink={({ href, ...props }) => <Link to={href} {...props} />}
      header={!collapsed && (
        <div className="project-switch">
          <Select
            label="当前项目"
            placeholder="未选择项目"
            value={active?.id ?? ""}
            disabled={!snapshot.value?.projects.length || projectBusy}
            options={(snapshot.value?.projects ?? []).map((p) => ({
              value: p.id,
              label: p.name,
            }))}
            onValueChange={(id) => {
              if (id !== active?.id)
                void changeProjects("projects_switch", { id })
                  .then(() => navigate(modules[0]?.path ?? "/projects"))
                  .catch((e) => show({ text: errorText(e), tone: "ng" }));
            }}
          />
        </div>
      )}
    />
  );
}
