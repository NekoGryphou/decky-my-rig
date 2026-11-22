import { ConfirmModal, TextField } from "@decky/ui";
import { useState } from "react";
import { CANCEL } from "../shared/product.messages";
import {
  PAIR,
  PAIRING_CODE_HELP,
  PAIRING_CODE_TITLE,
} from "./settings.messages";
import type { PairingCodeProps } from "./settings.types";
import { codeFieldStyle, inputStyle } from "./settings.styles";
import { pairingCode } from "./settings.utils";
import { PAIRING_CODE_LENGTH } from "./settings.consts";

export function PairingCode({ onSubmit, closeModal }: PairingCodeProps) {
  const [code, setCode] = useState("");
  return (
    <ConfirmModal
      strTitle={PAIRING_CODE_TITLE}
      strDescription={PAIRING_CODE_HELP}
      strOKButtonText={PAIR}
      strCancelButtonText={CANCEL}
      bOKDisabled={code.length !== PAIRING_CODE_LENGTH}
      onOK={() => {
        closeModal?.();
        onSubmit(code);
      }}
      onCancel={closeModal}
      onEscKeypress={closeModal}
    >
      <div style={codeFieldStyle}>
        <TextField
          aria-label={PAIRING_CODE_TITLE}
          style={inputStyle}
          mustBeNumeric
          value={code}
          onChange={(event) => setCode(pairingCode(event.currentTarget.value))}
        />
      </div>
    </ConfirmModal>
  );
}
