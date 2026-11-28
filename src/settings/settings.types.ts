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

/** The editor's state, refs and setters; a new object only when state changes. */
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
  // * The newest state: callbacks that outlive a render (a modal's submit) read it here instead of
  // * the render that created them.
  latest: RefObject<SettingsEditorState | null>;
  setForm: Setter<FormState>;
  setStep: Setter<EditorStep>;
  setLoad: Setter<EditorLoad>;
  setOperation: Setter<SettingsOperation>;
  setPaired: Setter<boolean>;
  setError: Setter<string>;
  setMacError: Setter<string>;
}

/** What the editor's controls call; every function keeps its identity. */
export interface EditorActions {
  setField: (name: keyof FormState, value: string) => void;
  detect: () => void;
  pairAgain: () => void;
  remove: () => void;
  submit: () => void;
  leave: () => void;
}

export interface HeaderActionsProps {
  editingId: string | null;
  name: string;
  disabled: boolean;
  actions: EditorActions;
}

export interface EditorFieldsProps {
  form: FormState;
  step: EditorStep;
  editingId: string | null;
  paired: boolean;
  macError: string;
  disabled: boolean;
  actions: EditorActions;
}

export interface FormActionsProps {
  step: EditorStep;
  editingId: string | null;
  operation: SettingsOperation;
  disabled: boolean;
  actions: EditorActions;
}

export interface TextRowProps {
  label: string;
  name: keyof FormState;
  value: string;
  disabled: boolean;
  numeric?: boolean;
  locked?: boolean;
  onField: EditorActions["setField"];
}

export interface MacRowProps {
  value: string;
  macError: string;
  disabled: boolean;
  onField: EditorActions["setField"];
  onDetect: EditorActions["detect"];
}
