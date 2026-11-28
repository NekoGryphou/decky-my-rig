import { act, render } from "@testing-library/react";
import type {
  ButtonHTMLAttributes,
  HTMLAttributes,
  PropsWithChildren,
  ReactElement,
  ReactNode,
} from "react";
import { cloneElement, forwardRef } from "react";
import { vi } from "vitest";
import { RPC_METHODS } from "./api.consts";
import type { PcSnapshot, SnapshotFrame } from "./api.types";

export const mocks = {
  routes: [] as string[],
  watch: vi.fn(),
  settings: vi.fn(),
  begin: vi.fn(),
  requestCode: vi.fn(),
  save: vi.fn(),
  cancel: vi.fn(),
  detect: vi.fn(),
  update: vi.fn(),
  remove: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
  attention: vi.fn(),
  addRoute: vi.fn(),
  removeRoute: vi.fn(),
  navigate: vi.fn(),
  navigateBack: vi.fn(),
  modal: vi.fn(),
  toast: vi.fn(),
  // - Called with the label each time a Field renders: tests count re-renders.
  fieldRender: vi.fn(),
};

// noinspection JSUnusedGlobalSymbols
export const deckyApi = {
  callable: (route: string) => {
    mocks.routes.push(route);
    return (
      {
        [RPC_METHODS.watchSnapshot]: mocks.watch,
        [RPC_METHODS.fetchSettings]: mocks.settings,
        [RPC_METHODS.beginPairing]: mocks.begin,
        [RPC_METHODS.requestPairingCode]: mocks.requestCode,
        [RPC_METHODS.savePairing]: mocks.save,
        [RPC_METHODS.cancelPairing]: mocks.cancel,
        [RPC_METHODS.detectMac]: mocks.detect,
        [RPC_METHODS.updatePc]: mocks.update,
        [RPC_METHODS.deletePc]: mocks.remove,
        [RPC_METHODS.startPc]: mocks.start,
        [RPC_METHODS.stopPc]: mocks.stop,
        [RPC_METHODS.setAttention]: mocks.attention,
      } as Record<string, ReturnType<typeof vi.fn>>
    )[route];
  },
  definePlugin: (factory: () => unknown) => factory,
  routerHook: { addRoute: mocks.addRoute, removeRoute: mocks.removeRoute },
  toaster: { toast: mocks.toast },
};

const dialogButton = ({
  children,
  onGamepadDirection: _onGamepadDirection,
  ...props
}: PropsWithChildren<ButtonHTMLAttributes<HTMLButtonElement>> & {
  onGamepadDirection?: unknown;
}) => <button {...props}>{children}</button>;

// noinspection JSUnusedGlobalSymbols
export const deckyUi = {
  staticClasses: { Title: "title" },
  gamepadUIClasses: { HeaderHeightVisible: "40px" },
  gamepadDialogClasses: { GamepadDialogContent: "gamepad-dialog-content" },
  DialogBody: ({
    children,
    ...props
  }: PropsWithChildren<HTMLAttributes<HTMLDivElement>>) => (
    <div {...props}>{children}</div>
  ),
  DialogHeader: ({ children }: PropsWithChildren) => <h1>{children}</h1>,
  DialogControlsSection: ({ children }: PropsWithChildren) => (
    <div>{children}</div>
  ),
  ConfirmModal: ({
    strTitle,
    strDescription,
    strOKButtonText,
    strCancelButtonText,
    bOKDisabled,
    onOK,
    onCancel,
    children,
  }: {
    strTitle?: string;
    strDescription?: string;
    strOKButtonText?: string;
    strCancelButtonText?: string;
    bOKDisabled?: boolean;
    onOK?: () => void;
    onCancel?: () => void;
  } & PropsWithChildren<Record<string, unknown>>) => (
    <div data-testid="confirm-modal">
      <h3>{strTitle}</h3>
      <p>{strDescription}</p>
      {children}
      <button disabled={bOKDisabled} onClick={onOK}>
        {strOKButtonText}
      </button>
      <button onClick={onCancel}>{strCancelButtonText}</button>
    </div>
  ),
  DialogButton: dialogButton,
  DialogButtonPrimary: dialogButton,
  Focusable: forwardRef<
    HTMLDivElement,
    PropsWithChildren<HTMLAttributes<HTMLDivElement>> & {
      "flow-children"?: string;
      onGamepadDirection?: unknown;
    }
  >(({ children, onGamepadDirection: _onGamepadDirection, ...props }, ref) => (
    <div ref={ref} {...props}>
      {children}
    </div>
  )),
  Field: ({
    label,
    children,
    bottomSeparator: _bottomSeparator,
    description,
    childrenContainerWidth: _childrenContainerWidth,
    inlineWrap: _inlineWrap,
    focusable: _focusable,
    icon: _icon,
  }: {
    label?: ReactNode;
    bottomSeparator?: string;
    description?: ReactNode;
    childrenContainerWidth?: string;
    inlineWrap?: string;
    focusable?: boolean;
    icon?: ReactNode;
  } & PropsWithChildren) => {
    mocks.fieldRender(label);
    return (
      <div data-field="">
        {label && (
          <label>
            {label}
            {children}
          </label>
        )}
        {description && <small>{description}</small>}
      </div>
    );
  },
  PanelSection: ({
    title,
    children,
  }: PropsWithChildren<{ title?: string }>) => (
    <section aria-label={title}>{children}</section>
  ),
  PanelSectionRow: ({ children }: PropsWithChildren) => <div>{children}</div>,
  TextField: ({
    mustBeNumeric: _mustBeNumeric,
    ...props
  }: {
    mustBeNumeric?: boolean;
  } & PropsWithChildren<Record<string, unknown>>) => (
    <input data-testid="text-field" {...props} />
  ),
  Navigation: {
    Navigate: mocks.navigate,
    NavigateBack: mocks.navigateBack,
    CloseSideMenus: vi.fn(),
  },
  showModal: mocks.modal,
};

export function backendError(message: string) {
  return Object.assign(new Error(""), {
    name: "Python Exception",
    pythonTraceback: `Traceback (most recent call last):\n  File "main.py", line 1\nException: ${message}\n`,
  });
}

export const TOKEN = "ab".repeat(32);

export const pc = {
  id: "11111111-1111-4111-8111-111111111111",
  name: "Living room PC",
  host: "192.168.1.40",
  port: 47991,
  mac: "02:00:00:00:00:01",
};

export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((accept, decline) => {
    resolve = accept;
    reject = decline;
  });
  return { promise, resolve, reject };
}

/** Renders the modal passed to the most recent showModal call. */
export function renderLastModal() {
  const element = mocks.modal.mock.lastCall?.[0] as ReactElement<{
    closeModal?: () => void;
  }>;
  const { results } = mocks.modal.mock;
  const close = results[results.length - 1]?.value.Close as () => void;
  // noinspection TypeScriptValidateTypes
  return render(cloneElement(element, { closeModal: close }));
}

let snapshotRequests: Array<{ resolve: (value: SnapshotFrame) => void }> = [];
let snapshotRevision = 0;
export async function emitPowerSnapshot(
  pcs: PcSnapshot[],
  count = 0,
  name: string | null = null,
) {
  const pending = snapshotRequests.shift();
  if (!pending) throw new Error("no snapshot subscription");
  await act(async () => {
    pending.resolve({
      revision: ++snapshotRevision,
      refreshed: true,
      pcs,
      test_stops: { count, name },
      closed: false,
    });
    await Promise.resolve();
  });
}
/** Ends every pending snapshot watch, as the backend does on unload. */
export async function releaseSnapshots() {
  for (const pending of snapshotRequests)
    pending.resolve({
      revision: 0,
      refreshed: false,
      pcs: [],
      test_stops: { count: 0, name: null },
      closed: true,
    });
  await Promise.resolve();
}

/** Resets every mock to its default reply. */
export function resetDeckyMocks() {
  mocks.begin.mockReset().mockImplementation(async (draft_id: string) => ({
    draft_id,
    name: "Fixture PC",
    mac: pc.mac,
    mac_error: null,
  }));
  mocks.save.mockReset().mockResolvedValue(pc);
  mocks.requestCode.mockReset().mockResolvedValue(undefined);
  mocks.cancel.mockReset().mockResolvedValue(undefined);
  mocks.detect.mockReset().mockResolvedValue(pc.mac);
  mocks.attention.mockReset().mockResolvedValue(undefined);
  snapshotRevision = 0;
  snapshotRequests = [];
  mocks.watch.mockReset().mockImplementation(() => {
    const waiting = deferred<SnapshotFrame>();
    snapshotRequests.push(waiting);
    return waiting.promise;
  });
  mocks.settings.mockReset().mockResolvedValue([]);
  mocks.update.mockReset().mockResolvedValue(pc);
  mocks.remove.mockReset().mockResolvedValue({ unpaired: true });
  mocks.start.mockReset().mockResolvedValue(undefined);
  mocks.stop.mockReset().mockResolvedValue(false);
  mocks.addRoute.mockReset();
  mocks.removeRoute.mockReset();
  mocks.navigate.mockReset();
  mocks.navigateBack.mockReset();
  mocks.modal.mockReset().mockImplementation(() => ({ Close: vi.fn() }));
  mocks.toast.mockReset();
  mocks.fieldRender.mockReset();
  vi.restoreAllMocks();
}
