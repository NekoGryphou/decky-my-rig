import { useSyncExternalStore } from "react";
import { EditorNavigation } from "../shared/navigation.service";
import { SettingsEditor } from "./settings-editor.view";

// noinspection HtmlUnknownAttribute
export function SettingsEditorPage() {
  const editingId = useSyncExternalStore(
    EditorNavigation.subscribe,
    EditorNavigation.getSnapshot,
  );
  return <SettingsEditor key={editingId ?? "new"} editingId={editingId} />;
}
