import { DialogControlsSection } from "@decky/ui";
import type { SettingsEditorProps } from "./settings.types";
import { submit, useSettingsEditor } from "./settings-editor.service";
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
  const editor = useSettingsEditor(editingId);
  const disabled = editor.operation !== null || editor.load !== "ready";
  const part = { editor, disabled };
  return (
    <SettingsLayout
      title={editingId ? TITLE_EDIT : TITLE_PAIR}
      action={editor.step === "details" && <HeaderActions {...part} />}
    >
      {editor.error && <AlertView message={editor.error} />}
      <form
        aria-label={formLabel(editor.step, editingId)}
        onSubmit={(event) => {
          event.preventDefault();
          submit(editor);
        }}
      >
        <DialogControlsSection>
          <EditorFields {...part} />
          <FormActions {...part} />
        </DialogControlsSection>
      </form>
    </SettingsLayout>
  );
}
