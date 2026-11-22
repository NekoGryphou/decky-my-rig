import {
  DialogBody,
  DialogHeader,
  Focusable,
  gamepadDialogClasses,
} from "@decky/ui";
import type { SettingsLayoutProps } from "./settings.types";
import {
  bodyStyle,
  contentStyle,
  headerStyle,
  pageStyle,
} from "./settings.styles";

export function SettingsLayout({
  title,
  action,
  children,
}: SettingsLayoutProps) {
  return (
    <div style={pageStyle}>
      <Focusable
        className={gamepadDialogClasses.GamepadDialogContent}
        style={contentStyle}
        flow-children="column"
      >
        <Focusable style={headerStyle} flow-children="row">
          <DialogHeader>{title}</DialogHeader>
          {action}
        </Focusable>
        <DialogBody style={bodyStyle}>{children}</DialogBody>
      </Focusable>
    </div>
  );
}
