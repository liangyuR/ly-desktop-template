import type { ComponentType } from "react";
import { LayoutDashboard, type LucideIcon } from "lucide-react";
import { ExamplePage } from "../pages/ExamplePage";

export interface PageModule {
  path: string;
  title: string;
  icon: LucideIcon;
  page: ComponentType;
}
export const modules: PageModule[] = [
  {
    path: "/workspace",
    title: "工作台",
    icon: LayoutDashboard,
    page: ExamplePage,
  },
];
