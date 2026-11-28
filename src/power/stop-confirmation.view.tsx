import { ConfirmModal } from "@decky/ui";
import { useEffect } from "react";
import { CANCEL } from "../shared/product.messages";
import { POWER_LABELS, STOP_WARNING, stopTitle } from "./power.messages";
import { powerService } from "./power.service";
import type { StopConfirmationProps } from "./power.types";

export function StopConfirmation({
  pc,
  closeModal,
  onDone,
}: StopConfirmationProps) {
  // ? Unmounting covers every way the question closes, including the B button.
  useEffect(() => () => onDone?.(), [onDone]);
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
