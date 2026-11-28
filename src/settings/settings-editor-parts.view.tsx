import {
  DialogButton,
  DialogButtonPrimary,
  Focusable,
  TextField,
  showModal,
} from "@decky/ui";
import { memo } from "react";
import type {
  EditorFieldsProps,
  FormActionsProps,
  HeaderActionsProps,
  MacRowProps,
  TextRowProps,
} from "./settings.types";
import { submitLabel } from "./settings.utils";
import { LabeledField } from "./labeled-field.view";
import { DeleteConfirmation } from "./delete-confirmation.view";
import { AlertView } from "../shared/alert.view";
import { CANCEL } from "../shared/product.messages";
import {
  DELETE_PC,
  DETECT,
  IP_ADDRESS,
  MAC_ADDRESS,
  PAIR_AGAIN,
  PC_NAME,
  PORT,
  deleteLabel,
} from "./settings.messages";
import {
  actionsStyle,
  buttonStyle,
  compactButtonStyle,
  growStyle,
  headerActionsStyle,
  inlineControlStyle,
  inputStyle,
} from "./settings.styles";

/** Pair again, and Delete for a saved PC. */
// noinspection HtmlUnknownAttribute
export function HeaderActions({
  editingId,
  name,
  disabled,
  actions,
}: HeaderActionsProps) {
  return (
    <Focusable style={headerActionsStyle} flow-children="row">
      <DialogButton
        style={compactButtonStyle}
        disabled={disabled}
        onClick={actions.pairAgain}
      >
        {PAIR_AGAIN}
      </DialogButton>
      {editingId && (
        <DialogButton
          style={compactButtonStyle}
          aria-label={deleteLabel(name)}
          disabled={disabled}
          onClick={() =>
            showModal(
              <DeleteConfirmation name={name} onDelete={actions.remove} />,
            )
          }
        >
          {DELETE_PC}
        </DialogButton>
      )}
    </Focusable>
  );
}

// * Memoised: a keystroke changes one value, and the other rows keep their props, so they don't
// * render. noinspection HtmlUnknownAttribute
const TextRow = memo(function TextRow({
  label,
  name,
  value,
  disabled,
  numeric = false,
  locked = false,
  onField,
}: TextRowProps) {
  return (
    <LabeledField label={label}>
      <TextField
        style={inputStyle}
        mustBeNumeric={numeric}
        value={value}
        disabled={disabled || locked}
        onChange={(event) => onField(name, event.currentTarget.value)}
      />
    </LabeledField>
  );
});

// noinspection HtmlUnknownAttribute
const MacRow = memo(function MacRow({
  value,
  macError,
  disabled,
  onField,
  onDetect,
}: MacRowProps) {
  return (
    <>
      <LabeledField label={MAC_ADDRESS}>
        <Focusable style={inlineControlStyle} flow-children="row">
          <div style={growStyle}>
            <TextField
              style={inputStyle}
              value={value}
              disabled={disabled}
              onChange={(event) => onField("mac", event.currentTarget.value)}
            />
          </div>
          <DialogButton
            style={buttonStyle}
            disabled={disabled}
            onClick={(event) => {
              event.preventDefault();
              onDetect();
            }}
          >
            {DETECT}
          </DialogButton>
        </Focusable>
      </LabeledField>
      {macError && <AlertView message={macError} />}
    </>
  );
});

/** The fields of this step: the address first, then the PC's details. */
// noinspection HtmlUnknownAttribute
export function EditorFields({
  form,
  step,
  editingId,
  paired,
  macError,
  disabled,
  actions,
}: EditorFieldsProps) {
  const details = step === "details";
  const showAddress = step === "connection" || editingId;
  // ! A re-paired PC's new credential belongs to the address it paired at.
  const locked = Boolean(editingId) && paired;
  const row = { disabled, onField: actions.setField };
  return (
    <>
      {details && (
        <TextRow {...row} label={PC_NAME} name="name" value={form.name} />
      )}
      {showAddress && (
        <>
          <TextRow
            {...row}
            label={IP_ADDRESS}
            name="host"
            value={form.host}
            locked={locked}
          />
          <TextRow
            {...row}
            label={PORT}
            name="port"
            value={form.port}
            numeric
            locked={locked}
          />
        </>
      )}
      {details && (
        <MacRow
          {...row}
          value={form.mac}
          macError={macError}
          onDetect={actions.detect}
        />
      )}
    </>
  );
}

/** Next or Save, and Cancel, which stays usable except while writing. */
// noinspection HtmlUnknownAttribute
export function FormActions({
  step,
  editingId,
  operation,
  disabled,
  actions,
}: FormActionsProps) {
  const writing = operation === "save" || operation === "delete";
  return (
    <Focusable style={actionsStyle} flow-children="row">
      <DialogButtonPrimary
        disabled={disabled}
        onClick={(event) => {
          event.preventDefault();
          actions.submit();
        }}
      >
        {submitLabel(step, editingId)}
      </DialogButtonPrimary>
      <DialogButton
        disabled={writing}
        onClick={(event) => {
          event.preventDefault();
          actions.leave();
        }}
      >
        {CANCEL}
      </DialogButton>
    </Focusable>
  );
}
