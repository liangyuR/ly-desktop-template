import { useState } from "react";
import { Alert } from "qing-steel";
import {
  HashRouter,
  Link,
  Navigate,
  Route,
  Routes,
  useLocation,
} from "react-router-dom";
import { AppProvider, useApp } from "./state";
import { modules } from "./modules";
import { desktop } from "../desktop";
import { Sidebar } from "../layout/Sidebar";
import { Frame } from "../layout/Frame";
import { ErrorBoundary } from "../layout/ErrorBoundary";
import { ProjectsPage } from "../pages/ProjectsPage";
import { SettingsPage } from "../pages/SettingsPage";
import { LicensePage } from "../pages/LicensePage";
import { AboutPage } from "../pages/AboutPage";

function Home() {
  const { projects, active } = useApp();
  if (desktop && !projects.value && !projects.error)
    return <p role="status">正在打开工作区…</p>;
  return (
    <Navigate
      replace
      to={active && modules.length ? modules[0].path : "/projects"}
    />
  );
}
function Messages() {
  const { license, update, settings } = useApp();
  const { pathname } = useLocation();
  const [dismissedUpdate, dismiss] = useState<string>();
  const state = license.value?.state;
  return (
    <>
      {!desktop && (
        <Alert tone="info">
          浏览器预览 · 项目文件、本机授权及系统服务需要桌面应用。
        </Alert>
      )}
      {settings.error && pathname !== "/settings" && (
        <Alert tone="warn" action={<Link to="/settings">设置</Link>}>
          {settings.error}
        </Alert>
      )}
      {license.error && pathname !== "/license" && (
        <Alert tone="warn" action={<Link to="/license">查看授权</Link>}>
          无法读取授权状态：{license.error}
        </Alert>
      )}
      {state &&
        state !== "valid" &&
        state !== "unconfigured" &&
        pathname !== "/license" && (
          <Alert tone="warn" action={<Link to="/license">查看授权</Link>}>
            {license.value?.message}
          </Alert>
        )}
      {update.value?.status === "available" &&
        dismissedUpdate !== update.value.version &&
        pathname !== "/about" && (
          <Alert
            tone="info"
            action={<Link to="/about">查看更新</Link>}
            onClose={() => dismiss(update.value?.version)}
          >
            发现新版本 {update.value.version}
          </Alert>
        )}
    </>
  );
}
function Shell() {
  const location = useLocation();
  return (
    <Frame>
      <div className="shell">
        <Sidebar />
        <main id="main-content" className="main-content">
          <div className="page-content">
            <Messages />
            <ErrorBoundary key={location.pathname}>
              <Routes>
                <Route path="/" element={<Home />} />
                <Route path="/projects" element={<ProjectsPage />} />
                {modules.map(({ path, page: Page }) => (
                  <Route key={path} path={path} element={<Page />} />
                ))}
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/license" element={<LicensePage />} />
                <Route path="/about" element={<AboutPage />} />
                <Route path="*" element={<Navigate to="/" replace />} />
              </Routes>
            </ErrorBoundary>
          </div>
        </main>
      </div>
    </Frame>
  );
}
export default function App() {
  return (
    <AppProvider>
      <HashRouter>
        <Shell />
      </HashRouter>
    </AppProvider>
  );
}
