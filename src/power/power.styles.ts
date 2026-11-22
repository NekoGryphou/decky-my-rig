import type { CSSProperties } from "react";

export const actionStyle: CSSProperties = {
  width: "96px",
  minWidth: 0,
  padding: "6px 0",
};

export const rowActionStyle: CSSProperties = {
  ...actionStyle,
  marginTop: "4px",
};
