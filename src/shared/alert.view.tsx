import { Field } from "@decky/ui";
import { FaExclamationTriangle } from "react-icons/fa";
import type { AlertViewProps } from "./alert.types";

export function AlertView({ message }: AlertViewProps) {
  return (
    <Field
      focusable={false}
      bottomSeparator="none"
      icon={<FaExclamationTriangle />}
      label={<span role="alert">{message}</span>}
    />
  );
}
