import { useState } from "react";
import { NavLink, useNavigate } from "react-router-dom";
import {
  FolderKanban,
  Info,
  KeyRound,
  PanelLeftClose,
  PanelLeftOpen,
  Settings,
  type LucideIcon,
} from "lucide-react";
import { Select, useSnackbar } from "qing-steel";
import { useApp } from "../app/state";
import { modules } from "../app/modules";
import { errorText } from "../desktop";

const footer = [
  { path: "/settings", title: "设置", icon: Settings },
  { path: "/license", title: "软件授权", icon: KeyRound },
  { path: "/about", title: "关于", icon: Info },
];
const projects = { path: "/projects", title: "项目", icon: FolderKanban };
function Entry({
  entry,
  collapsed,
}: {
  entry: { path: string; title: string; icon: LucideIcon };
  collapsed: boolean;
}) {
  return (
    <NavLink
      className={({ isActive }) => "nav-item" + (isActive ? " active" : "")}
      to={entry.path}
      aria-label={entry.title}
      title={collapsed ? entry.title : undefined}
    >
      <entry.icon size={17} />
      {!collapsed && <span>{entry.title}</span>}
    </NavLink>
  );
}
export function Sidebar() {
  const { projects: snapshot, active, changeProjects, projectBusy } = useApp();
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const { show } = useSnackbar();
  return (
    <aside className={"sidebar" + (collapsed ? " rail" : "")}>
      {!collapsed && (
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
      <nav aria-label="工作区" className="nav-group">
        {!collapsed && <span className="group-title">工作区</span>}
        <Entry entry={projects} collapsed={collapsed} />
        {modules.map((m) => (
          <Entry key={m.path} entry={m} collapsed={collapsed} />
        ))}
      </nav>
      <nav aria-label="应用" className="nav-group nav-bottom">
        {!collapsed && <span className="group-title">应用</span>}
        {footer.map((item) => (
          <Entry key={item.path} entry={item} collapsed={collapsed} />
        ))}
      </nav>
      <button
        type="button"
        className="rail-toggle"
        aria-label={collapsed ? "展开侧栏" : "收起侧栏"}
        aria-expanded={!collapsed}
        onClick={() => setCollapsed((v) => !v)}
      >
        {collapsed ? (
          <PanelLeftOpen />
        ) : (
          <>
            <PanelLeftClose />
            <span>收起侧栏</span>
          </>
        )}
      </button>
    </aside>
  );
}
