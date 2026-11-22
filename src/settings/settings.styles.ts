import type { CSSProperties } from "react";
import { gamepadUIClasses } from "@decky/ui";
export const pageStyle: CSSProperties = {
  height: "100%",
  boxSizing: "border-box",
  paddingBlock: gamepadUIClasses.HeaderHeightVisible,
  display: "flex",
};
export const contentStyle: CSSProperties = {
  boxSizing: "border-box",
  width: "100%",
  maxWidth: "720px",
  marginInline: "auto",
  minHeight: 0,
  display: "flex",
  flexDirection: "column",
};
export const bodyStyle: CSSProperties = {
  minHeight: 0,
  overflowY: "auto",
  flex: 1,
};
export const actionsStyle: CSSProperties = {
  display: "flex",
  gap: "12px",
  marginTop: "24px",
};
export const headerActionsStyle: CSSProperties = {
  display: "flex",
  gap: "12px",
};
// ? One width for every label so the inputs start at the same position.
export const labelStyle: CSSProperties = {
  display: "inline-block",
  width: "140px",
};
export const compactButtonStyle: CSSProperties = {
  width: "auto",
  minWidth: "160px",
};
export const buttonStyle: CSSProperties = {
  width: "auto",
  minWidth: "120px",
  flexShrink: 0,
};
export const headerStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: "12px",
  marginBottom: "16px",
};
export const emptyStyle: CSSProperties = {
  height: "100%",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  opacity: 0.6,
};
export const inputStyle: CSSProperties = { width: "100%", minWidth: 0 };
export const inlineControlStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "12px",
  width: "100%",
};
export const growStyle: CSSProperties = { flex: 1, minWidth: 0 };
export const codeFieldStyle: CSSProperties = { marginTop: "16px" };
