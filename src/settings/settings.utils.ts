import { publicMessage } from "../shared/error.utils";
import type { EditorStep, FormState } from "./settings.types";
import { DEFAULT_PORT, PAIRING_CODE_LENGTH } from "./settings.consts";
import {
  FORM_CONNECTION,
  NEXT,
  SAVE,
  SAVE_PC,
  SETTINGS_ERROR,
  TITLE_EDIT,
} from "./settings.messages";
import { ADD_PC } from "../shared/product.messages";
export const blankForm = (): FormState => ({
  name: "",
  host: "",
  port: String(DEFAULT_PORT),
  mac: "",
});
export function settingsError(error: unknown): string {
  return publicMessage(error, SETTINGS_ERROR);
}
export function pairingCode(value: string): string {
  return value.replace(/[^0-9]/g, "").slice(0, PAIRING_CODE_LENGTH);
}
/** The accessible name of the editor's form at this step. */
export function formLabel(step: EditorStep, editingId: string | null): string {
  if (step === "connection") return FORM_CONNECTION;
  return editingId ? TITLE_EDIT : ADD_PC;
}
/** The primary button's label at this step. */
export function submitLabel(
  step: EditorStep,
  editingId: string | null,
): string {
  if (step === "connection") return NEXT;
  return editingId ? SAVE : SAVE_PC;
}
