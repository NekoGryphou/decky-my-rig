import { ConfirmModal } from "@decky/ui";
import { CANCEL } from "../shared/product.messages";
import { POWER_LABELS, STOP_WARNING, stopTitle } from "./power.messages";
import { powerService } from "./power.service";
import type { StopConfirmationProps } from "./power.types";

export function StopConfirmation({ pc, closeModal }: StopConfirmationProps) {
  return (
    <ConfirmModal
      strTitle={stopTitle(pc.name)}
      strDescription={STOP_WARNING}
      strOKButtonText={POWER_LABELS.stop}
      strCancelButtonText={CANCEL}
      bDestructiveWarning
      onOK={() => {
        closeModal?.();
        void powerService.act(pc.id, "stop");
      }}
      onCancel={closeModal}
      onEscKeypress={closeModal}
    />
  );
}
