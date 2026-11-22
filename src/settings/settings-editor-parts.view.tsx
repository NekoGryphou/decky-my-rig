import {
  DialogButton,
  DialogButtonPrimary,
  Focusable,
  TextField,
  showModal,
} from "@decky/ui";
import type { EditorPartProps, TextRowProps } from "./settings.types";
import {
  detect,
  leave,
  pairAgain,
  remove,
  setField,
  submit,
} from "./settings-editor.service";
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
export function HeaderActions({ editor, disabled }: EditorPartProps) {
  const { editingId, form } = editor;
  return (
    <Focusable style={headerActionsStyle} flow-children="row">
      <DialogButton
        style={compactButtonStyle}
        disabled={disabled}
        onClick={() => pairAgain(editor)}
      >
        {PAIR_AGAIN}
      </DialogButton>
      {editingId && (
        <DialogButton
          style={compactButtonStyle}
          aria-label={deleteLabel(form.name)}
          disabled={disabled}
          onClick={() =>
            showModal(
              <DeleteConfirmation
                name={form.name}
                onDelete={() => {
                  void remove(editor);
                }}
              />,
            )
          }
        >
          {DELETE_PC}
        </DialogButton>
      )}
    </Focusable>
  );
}

// noinspection HtmlUnknownAttribute
function TextRow({
  editor,
  disabled,
  label,
  name,
  numeric = false,
  locked = false,
}: TextRowProps) {
  return (
    <LabeledField label={label}>
      <TextField
        style={inputStyle}
        mustBeNumeric={numeric}
        value={editor.form[name]}
        disabled={disabled || locked}
        onChange={(event) => setField(editor, name, event.currentTarget.value)}
      />
    </LabeledField>
  );
}

// noinspection HtmlUnknownAttribute
function MacRow({ editor, disabled }: EditorPartProps) {
  return (
    <>
      <LabeledField label={MAC_ADDRESS}>
        <Focusable style={inlineControlStyle} flow-children="row">
          <div style={growStyle}>
            <TextField
              style={inputStyle}
              value={editor.form.mac}
              disabled={disabled}
              onChange={(event) =>
                setField(editor, "mac", event.currentTarget.value)
              }
            />
          </div>
          <DialogButton
            style={buttonStyle}
            disabled={disabled}
            onClick={(event) => {
              event.preventDefault();
              void detect(editor);
            }}
          >
            {DETECT}
          </DialogButton>
        </Focusable>
      </LabeledField>
      {editor.macError && <AlertView message={editor.macError} />}
    </>
  );
}

/** The fields of this step: the address first, then the PC's details. */
// noinspection HtmlUnknownAttribute
export function EditorFields({ editor, disabled }: EditorPartProps) {
  const details = editor.step === "details";
  const showAddress = editor.step === "connection" || editor.editingId;
  // ! A re-paired PC's new credential belongs to the address it paired at.
  const locked = Boolean(editor.editingId) && editor.paired;
  const part = { editor, disabled };
  return (
    <>
      {details && <TextRow {...part} label={PC_NAME} name="name" />}
      {showAddress && (
        <>
          <TextRow {...part} label={IP_ADDRESS} name="host" locked={locked} />
          <TextRow {...part} label={PORT} name="port" numeric locked={locked} />
        </>
      )}
      {details && <MacRow {...part} />}
    </>
  );
}

/** Next or Save, and Cancel, which stays usable except while writing. */
// noinspection HtmlUnknownAttribute
export function FormActions({ editor, disabled }: EditorPartProps) {
  const writing = editor.operation === "save" || editor.operation === "delete";
  return (
    <Focusable style={actionsStyle} flow-children="row">
      <DialogButtonPrimary
        disabled={disabled}
        onClick={(event) => {
          event.preventDefault();
          submit(editor);
        }}
      >
        {submitLabel(editor.step, editor.editingId)}
      </DialogButtonPrimary>
      <DialogButton
        disabled={writing}
        onClick={(event) => {
          event.preventDefault();
          leave(editor);
        }}
      >
        {CANCEL}
      </DialogButton>
    </Focusable>
  );
}
