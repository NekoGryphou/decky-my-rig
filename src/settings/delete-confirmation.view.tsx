import { ConfirmModal } from "@decky/ui";
import type { DeleteConfirmationProps } from "./settings.types";
import {
  DELETE,
  DELETE_WARNING,
  KEEP_PC,
  deleteTitle,
} from "./settings.messages";
export function DeleteConfirmation({
  name,
  onDelete,
  closeModal,
}: DeleteConfirmationProps) {
  return (
    <ConfirmModal
      strTitle={deleteTitle(name)}
      strDescription={DELETE_WARNING}
      strOKButtonText={DELETE}
      strCancelButtonText={KEEP_PC}
      bDestructiveWarning
      onOK={() => {
        closeModal?.();
        onDelete();
      }}
      onCancel={closeModal}
      onEscKeypress={closeModal}
    />
  );
}
