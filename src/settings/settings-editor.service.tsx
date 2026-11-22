import { Navigation, showModal } from "@decky/ui";
import { toaster } from "@decky/api";
import type { Dispatch, RefObject, SetStateAction } from "react";
import { useEffect, useRef, useState } from "react";
import {
  beginPairing,
  cancelPairing,
  deletePc,
  detectMac,
  fetchSettings,
  requestPairingCode,
  savePairing,
  updatePc,
} from "../shared/api.service";
import { PRODUCT_NAME } from "../shared/product.messages";
import type {
  EditorLoad,
  EditorStep,
  FormState,
  SettingsEditorState,
  SettingsOperation,
} from "./settings.types";
import { blankForm, settingsError } from "./settings.utils";
import {
  MAC_ERROR,
  PC_MISSING,
  SETTINGS_ERROR,
  notUnpaired,
} from "./settings.messages";
import { PairingCode } from "./pairing-code.view";

/** The editor's state for this render; it starts loading an existing PC. */
export function useSettingsEditor(
  editingId: string | null,
): SettingsEditorState {
  const [form, setForm] = useState<FormState>(blankForm);
  const [step, setStep] = useState<EditorStep>(
    editingId ? "details" : "connection",
  );
  const [load, setLoad] = useState<EditorLoad>(editingId ? "loading" : "ready");
  const [operation, setOperation] = useState<SettingsOperation>(null);
  const [paired, setPaired] = useState(false);
  const [error, setError] = useState("");
  const [macError, setMacError] = useState("");
  const alive = useRef(true);
  const working = useRef(false);
  const draft = useRef<string | null>(null);

  useEffect(() => {
    alive.current = true;
    if (editingId) loadPc(editingId, alive, setForm, setLoad, setError);
    return () => {
      alive.current = false;
      discard(draft);
    };
  }, [editingId]);

  return {
    editingId,
    form,
    step,
    load,
    operation,
    paired,
    error,
    macError,
    alive,
    working,
    draft,
    setForm,
    setStep,
    setLoad,
    setOperation,
    setPaired,
    setError,
    setMacError,
  };
}

function loadPc(
  editingId: string,
  alive: RefObject<boolean>,
  setForm: Dispatch<SetStateAction<FormState>>,
  setLoad: Dispatch<SetStateAction<EditorLoad>>,
  setError: Dispatch<SetStateAction<string>>,
) {
  void fetchSettings()
    .then((pcs) => {
      if (!alive.current) return;
      const pc = pcs.find((item) => item.id === editingId);
      if (!pc) {
        setError(PC_MISSING);
        setLoad("missing");
        return;
      }
      setForm({
        name: pc.name,
        host: pc.host,
        port: String(pc.port),
        mac: pc.mac,
      });
      setLoad("ready");
    })
    .catch(() => {
      if (!alive.current) return;
      setError(SETTINGS_ERROR);
      setLoad("missing");
    });
}

/** Cancels the unsaved pairing, if any. */
function discard(draft: RefObject<string | null>) {
  const id = draft.current;
  draft.current = null;
  if (id) void cancelPairing(id).catch(() => {});
}

export function setField(
  editor: SettingsEditorState,
  name: keyof FormState,
  value: string,
) {
  editor.setForm((current) => ({ ...current, [name]: value }));
}

export function leave(editor: SettingsEditorState) {
  editor.alive.current = false;
  discard(editor.draft);
  Navigation.NavigateBack();
}

/** Runs one user operation at a time; nothing touches state after unmount. */
async function run(
  editor: SettingsEditorState,
  kind: Exclude<SettingsOperation, null>,
  task: () => Promise<void>,
  onError: (failure: unknown) => void,
) {
  const { alive, working } = editor;
  if (working.current || !alive.current) return;
  working.current = true;
  editor.setOperation(kind);
  try {
    await task();
  } catch (failure) {
    if (alive.current) onError(failure);
  } finally {
    working.current = false;
    if (alive.current) editor.setOperation(null);
  }
}

async function askCode(editor: SettingsEditorState) {
  if (editor.load !== "ready") return;
  await run(
    editor,
    "request",
    async () => {
      editor.setError("");
      await requestPairingCode(editor.form.host, editor.form.port);
      if (editor.alive.current)
        showModal(<PairingCode onSubmit={(code) => void pair(editor, code)} />);
    },
    (failure) => editor.setError(settingsError(failure)),
  );
}

export function pairAgain(editor: SettingsEditorState) {
  discard(editor.draft);
  editor.setPaired(false);
  editor.setError("");
  editor.setMacError("");
  if (editor.editingId) void askCode(editor);
  else editor.setStep("connection");
}

async function pair(editor: SettingsEditorState, code: string) {
  if (editor.load !== "ready") return;
  const { editingId, draft } = editor;
  const id = crypto.randomUUID();
  await run(
    editor,
    "pair",
    async () => {
      editor.setError("");
      draft.current = id;
      const result = await beginPairing(
        id,
        editor.form.host,
        editor.form.port,
        code,
        editingId,
      );
      if (!editor.alive.current || draft.current !== id) {
        void cancelPairing(id).catch(() => {});
        return;
      }
      editor.setForm((current) => ({
        ...current,
        name: editingId ? current.name : result.name,
        // * A failed detection keeps the MAC an existing PC already has.
        mac: result.mac ?? (editingId ? current.mac : ""),
      }));
      editor.setMacError(result.mac_error ? MAC_ERROR : "");
      editor.setPaired(true);
      editor.setStep("details");
    },
    (failure) => {
      if (draft.current !== id) return;
      discard(draft);
      editor.setError(settingsError(failure));
    },
  );
}

async function save(editor: SettingsEditorState) {
  const { editingId, paired, draft, form } = editor;
  if (editor.load !== "ready" || (!editingId && !paired)) return;
  await run(
    editor,
    "save",
    async () => {
      editor.setError("");
      if (paired && draft.current) {
        await savePairing(draft.current, form.name, form.mac);
        // ! Saved: the draft is used up and mustn't be canceled on leave.
        draft.current = null;
      } else if (editingId) await updatePc(editingId, form);
      if (editor.alive.current) leave(editor);
    },
    // ? Keep the draft: fixing a field lets Save work again, and Pair again
    //   starts over when the backend says the pairing itself is gone.
    (failure) => editor.setError(settingsError(failure)),
  );
}

export async function detect(editor: SettingsEditorState) {
  await run(
    editor,
    "detect",
    async () => {
      editor.setMacError("");
      const mac = await detectMac(editor.form.host);
      if (editor.alive.current) setField(editor, "mac", mac);
    },
    () => editor.setMacError(MAC_ERROR),
  );
}

export async function remove(editor: SettingsEditorState) {
  const { editingId } = editor;
  if (!editingId) return;
  await run(
    editor,
    "delete",
    async () => {
      editor.setError("");
      const { unpaired } = await deletePc(editingId);
      if (!unpaired)
        toaster.toast({
          title: PRODUCT_NAME,
          body: notUnpaired(editor.form.name),
          showToast: true,
        });
      if (editor.alive.current) leave(editor);
    },
    (failure) => editor.setError(settingsError(failure)),
  );
}

/** Next asks the PC for a code; Save stores the PC. */
export function submit(editor: SettingsEditorState) {
  if (editor.step === "connection") void askCode(editor);
  else void save(editor);
}
