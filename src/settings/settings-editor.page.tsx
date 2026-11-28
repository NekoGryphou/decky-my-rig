import { useEffect, useSyncExternalStore } from "react";
import { attend } from "../shared/attention.service";
import { EditorNavigation } from "../shared/navigation.service";
import { SettingsEditor } from "./settings-editor.view";

// noinspection HtmlUnknownAttribute
export function SettingsEditorPage() {
  const editingId = useSyncExternalStore(
    EditorNavigation.subscribe,
    EditorNavigation.getSnapshot,
  );
  useEffect(() => attend(), []);
  return <SettingsEditor key={editingId ?? "new"} editingId={editingId} />;
}
