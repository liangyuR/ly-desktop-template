import { afterEach, beforeAll, beforeEach, expect, test, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom/vitest";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { ComponentType } from "react";
import type { Project } from "../src/desktop";

let App: ComponentType;
beforeAll(async () => {
  vi.stubGlobal("isTauri", true);
  vi.stubGlobal("matchMedia", () => ({
    matches: false,
    addEventListener() {},
    removeEventListener() {},
  }));
  App = (await import("../src/app/App")).default;
});
beforeEach(() => {
  window.history.replaceState(null, "", "#/projects");
  mockWindows("main");
});
afterEach(() => {
  cleanup();
  clearMocks();
});

function backend(
  override: Record<string, (args: Record<string, unknown>) => unknown> = {},
) {
  const records: Project[] = [];
  mockIPC(
    (command, payload) => {
      if (command in override)
        return override[command](payload as Record<string, unknown>);
      switch (command) {
        case "license_status":
          return {
            state: "expired",
            message: "授权已到期",
            configured: true,
            expiresAt: "2020-01-01T00:00:00Z",
          };
        case "get_settings":
          return { theme: "system" };
        case "projects_list":
          return { projects: records, activeId: null };
        case "projects_create": {
          const p = payload as { name: string; description: string };
          records.push({ id: "first", ...p });
          return { projects: records, activeId: "first" };
        }
        case "update_check":
          return { status: "current" };
        case "app_info":
          return { name: "测试应用", version: "1.0.0" };
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}

test("授权已过期 → 用户仍可创建并看到当前项目", async () => {
  backend();
  const user = userEvent.setup();
  render(<App />);
  expect(await screen.findByText("授权已到期")).toBeVisible();
  await user.click(screen.getByRole("button", { name: "新建项目" }));
  const dialog = within(await screen.findByRole("dialog"));
  await user.type(dialog.getByLabelText(/项目名称/), "首件检验");
  await user.click(dialog.getByRole("button", { name: "新建项目" }));
  const row = within(await screen.findByRole("listitem"));
  expect(row.getByText("首件检验")).toBeVisible();
  expect(row.getByText("当前项目")).toBeVisible();
});

test("用户暂缓已发现的更新 → 仍在当前工作页且不开始安装", async () => {
  let installed = false;
  backend({
    update_check: () => ({ status: "available", version: "2.0.0" }),
    update_install: () => {
      installed = true;
    },
  });
  const user = userEvent.setup();
  render(<App />);
  const banner = await screen.findByText("发现新版本 2.0.0");
  const alert = banner.closest("[role='status'],[role='alert'],.qs-alert")!;
  const dismiss = within(alert as HTMLElement).getByRole("button", {
    name: /关闭/,
  });
  await user.click(dismiss);
  expect(screen.queryByText("发现新版本 2.0.0")).toBeNull();
  expect(screen.getByRole("heading", { name: "项目" })).toBeVisible();
  expect(installed).toBe(false);
});

test("切换目标项目打开失败 → 当前项目与错误原因同时可见", async () => {
  backend({
    projects_list: () => ({
      projects: [
        { id: "a", name: "项目 A", description: "" },
        { id: "b", name: "项目 B", description: "" },
      ],
      activeId: "a",
    }),
    projects_switch: () => {
      throw new Error("项目目录不可用");
    },
  });
  const user = userEvent.setup();
  render(<App />);
  const target = within(
    (await within(screen.getByRole("main")).findByText("项目 B")).closest(
      "li",
    )!,
  );
  await user.click(target.getByRole("button", { name: "切换" }));
  expect(await screen.findByText("项目目录不可用")).toBeVisible();
  const active = screen
    .getByText("当前项目", { selector: ".active-tag" })
    .closest("li")!;
  expect(within(active).getByText("项目 A")).toBeVisible();
});
