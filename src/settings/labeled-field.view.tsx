import { Field } from "@decky/ui";
import type { LabeledFieldProps } from "./settings.types";
import { labelStyle } from "./settings.styles";

export function LabeledField({ label, children }: LabeledFieldProps) {
  return (
    <Field
      label={<span style={labelStyle}>{label}</span>}
      childrenContainerWidth="max"
    >
      {children}
    </Field>
  );
}
