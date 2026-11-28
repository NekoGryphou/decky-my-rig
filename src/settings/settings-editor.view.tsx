import { DialogControlsSection } from "@decky/ui";
import type { SettingsEditorProps } from "./settings.types";
import { useSettingsEditor } from "./settings-editor.service";
import {
  EditorFields,
  FormActions,
  HeaderActions,
} from "./settings-editor-parts.view";
import { formLabel } from "./settings.utils";
import { TITLE_EDIT, TITLE_PAIR } from "./settings.messages";
import { AlertView } from "../shared/alert.view";
import { SettingsLayout } from "./settings-layout.view";

// noinspection HtmlUnknownAttribute
export function SettingsEditor({ editingId }: SettingsEditorProps) {
  const { editor, actions } = useSettingsEditor(editingId);
  const disabled = editor.operation !== null || editor.load !== "ready";
  return (
    <SettingsLayout
      title={editingId ? TITLE_EDIT : TITLE_PAIR}
      action={
        editor.step === "details" && (
          <HeaderActions
            editingId={editingId}
            name={editor.form.name}
            disabled={disabled}
            actions={actions}
          />
        )
      }
    >
      {editor.error && <AlertView message={editor.error} />}
      <form
        aria-label={formLabel(editor.step, editingId)}
        onSubmit={(event) => {
          event.preventDefault();
          actions.submit();
        }}
      >
        <DialogControlsSection>
          <EditorFields
            form={editor.form}
            step={editor.step}
            editingId={editingId}
            paired={editor.paired}
            macError={editor.macError}
            disabled={disabled}
            actions={actions}
          />
          <FormActions
            step={editor.step}
            editingId={editingId}
            operation={editor.operation}
            disabled={disabled}
            actions={actions}
          />
        </DialogControlsSection>
      </form>
    </SettingsLayout>
  );
}
