import { createContext, useContext, type ReactNode } from "react";
import { createPortal } from "react-dom";
export const TitlebarSlots = createContext<{
  details: HTMLElement | null;
  actions: HTMLElement | null;
}>({ details: null, actions: null });
export function TitlebarContent({
  slot,
  children,
}: {
  slot: "details" | "actions";
  children: ReactNode;
}) {
  const target = useContext(TitlebarSlots)[slot];
  return target ? createPortal(children, target) : null;
}
