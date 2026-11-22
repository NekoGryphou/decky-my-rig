import type { Dispatch, ReactNode, RefObject, SetStateAction } from "react";

export interface SettingsLayoutProps {
  title: string;
  action?: ReactNode;
  children: ReactNode;
}

export interface FormState {
  name: string;
  host: string;
  port: string;
  mac: string;
}

export interface SettingsEditorProps {
  editingId: string | null;
}
export interface PairingCodeProps {
  onSubmit: (code: string) => void;
  closeModal?: () => void;
}
export interface DeleteConfirmationProps {
  name: string;
  onDelete: () => void;
  closeModal?: () => void;
}

export type SettingsOperation =
  "request" | "pair" | "save" | "detect" | "delete" | null;

export interface LabeledFieldProps {
  label: string;
  children: ReactNode;
}

export type EditorStep = "connection" | "details";
/** An existing PC's details load first; "missing" when they can't be read. */
export type EditorLoad = "loading" | "ready" | "missing";

type Setter<T> = Dispatch<SetStateAction<T>>;

/** One render of the editor: its state, refs and setters. */
export interface SettingsEditorState {
  editingId: string | null;
  form: FormState;
  step: EditorStep;
  load: EditorLoad;
  operation: SettingsOperation;
  paired: boolean;
  error: string;
  macError: string;
  alive: RefObject<boolean>;
  working: RefObject<boolean>;
  draft: RefObject<string | null>;
  setForm: Setter<FormState>;
  setStep: Setter<EditorStep>;
  setLoad: Setter<EditorLoad>;
  setOperation: Setter<SettingsOperation>;
  setPaired: Setter<boolean>;
  setError: Setter<string>;
  setMacError: Setter<string>;
}

export interface EditorPartProps {
  editor: SettingsEditorState;
  disabled: boolean;
}

export interface TextRowProps extends EditorPartProps {
  label: string;
  name: keyof FormState;
  numeric?: boolean;
  locked?: boolean;
}
