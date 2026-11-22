import "@testing-library/jest-dom/vitest";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import type { ReactElement } from "react";
import { cloneElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// noinspection TypeScriptUnresolvedReference
vi.mock(
  "@decky/api",
  async () => (await import("../shared/decky.fixtures")).deckyApi,
);
// noinspection TypeScriptUnresolvedReference
vi.mock(
  "@decky/ui",
  async () => (await import("../shared/decky.fixtures")).deckyUi,
);

import { powerService } from "../power/power.service";
import { SettingsPage } from "./settings.page";
import { SettingsEditorPage } from "./settings-editor.page";
import { EditorNavigation } from "../shared/navigation.service";
import { SETTINGS_EDITOR_ROUTE } from "../shared/routes.consts";
import {
  TOKEN,
  deferred,
  emitPowerSnapshot,
  mocks,
  pc,
  releaseSnapshots,
  renderLastModal,
  resetDeckyMocks,
  backendError,
} from "../shared/decky.fixtures";

afterEach(async () => {
  powerService.stop();
  await releaseSnapshots();
});
beforeEach(() => {
  EditorNavigation.reset();
  resetDeckyMocks();
});

describe("Settings routes", () => {
  function codeModal() {
    const element = mocks.modal.mock.lastCall?.[0] as ReactElement<{
      closeModal?: () => void;
    }>;
    // noinspection TypeScriptValidateTypes
    return render(cloneElement(element, { closeModal: vi.fn() }));
  }
  function enterCode(code = "12345678") {
    const modal = codeModal();
    fireEvent.change(screen.getByLabelText("Pairing code"), {
      target: { value: code },
    });
    fireEvent.click(screen.getByRole("button", { name: "Pair" }));
    modal.unmount();
  }
  async function openCodeModal(button: string) {
    const opened = mocks.modal.mock.calls.length;
    fireEvent.click(screen.getByRole("button", { name: button }));
    await waitFor(() => expect(mocks.modal).toHaveBeenCalledTimes(opened + 1));
  }
  async function connect(host = pc.host, code = "12345678") {
    fireEvent.change(screen.getByLabelText("IP address"), {
      target: { value: host },
    });
    await openCodeModal("Next");
    enterCode(code);
  }
  async function pair() {
    await connect();
    await screen.findByLabelText("PC name");
  }
  async function existing() {
    mocks.settings.mockResolvedValue([pc]);
    EditorNavigation.open(pc.id);
    const editor = render(<SettingsEditorPage />);
    await screen.findByDisplayValue(pc.name);
    return editor;
  }
  it("lists several PCs with status and address without exposing connection details", async () => {
    mocks.settings.mockResolvedValue([
      pc,
      { ...pc, id: "second", name: "Office PC", host: "192.168.1.41" },
    ]);
    powerService.start();
    await emitPowerSnapshot([{ id: pc.id, name: pc.name, state: "online" }]);
    render(<SettingsPage />);
    await screen.findByRole("button", { name: `Edit ${pc.name}` });
    expect(screen.getByText(`Online · ${pc.host}`)).toBeInTheDocument();
    expect(screen.getByText("Checking… · 192.168.1.41")).toBeInTheDocument();
    expect(screen.getAllByRole("button")).toHaveLength(3);
    expect(document.body.textContent).not.toContain(pc.mac);
    expect(document.body.textContent).not.toContain("47991");
    fireEvent.click(screen.getByRole("button", { name: "Add PC" }));
    expect(mocks.navigate).toHaveBeenCalledWith(SETTINGS_EDITOR_ROUTE);
  });
  it("says the list is loading, then that there is no PC", async () => {
    let resolve: (pcs: never[]) => void = () => {};
    mocks.settings.mockReturnValue(new Promise((done) => (resolve = done)));
    render(<SettingsPage />);
    expect(screen.getByText("Loading…")).toBeInTheDocument();
    resolve([]);
    await screen.findByText("No PC");
    expect(screen.queryByText("Loading…")).not.toBeInTheDocument();
  });
  it("pairs first, then confirms the name and MAC and saves the public fields", async () => {
    render(<SettingsEditorPage />);
    expect(screen.queryByLabelText("PC name")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("MAC address")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Port")).toHaveValue("47991");
    await pair();
    expect(mocks.begin).toHaveBeenCalledWith(
      expect.stringMatching(/^[0-9a-f-]{36}$/),
      pc.host,
      "47991",
      "12345678",
      null,
    );
    expect(screen.getByLabelText("PC name")).toHaveValue("Fixture PC");
    expect(screen.getByLabelText("MAC address")).toHaveValue(pc.mac);
    expect(mocks.save).not.toHaveBeenCalled();
    fireEvent.change(screen.getByLabelText("PC name"), {
      target: { value: "My PC" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save PC" }));
    await waitFor(() => expect(mocks.navigateBack).toHaveBeenCalledTimes(1));
    expect(mocks.save).toHaveBeenCalledWith(
      mocks.begin.mock.calls[0][0],
      "My PC",
      pc.mac,
    );
  });
  it("pairs on the edited port", async () => {
    render(<SettingsEditorPage />);
    fireEvent.change(screen.getByLabelText("Port"), {
      target: { value: "48000" },
    });
    await pair();
    expect(mocks.begin.mock.calls[0][2]).toBe("48000");
  });
  it("permits manual MAC entry and Detect again without another pairing exchange", async () => {
    mocks.begin.mockImplementation(async (draft_id: string) => ({
      draft_id,
      name: "Fixture PC",
      mac: null,
      mac_error: "MAC detection failed. Enter the MAC address manually.",
    }));
    render(<SettingsEditorPage />);
    await pair();
    expect(screen.getByRole("alert")).toHaveTextContent("MAC detection failed");
    fireEvent.change(screen.getByLabelText("MAC address"), {
      target: { value: "020000000002" },
    });
    mocks.detect.mockRejectedValueOnce(new Error(TOKEN));
    fireEvent.click(screen.getByText("Detect"));
    await waitFor(() => expect(screen.getByText("Detect")).toBeEnabled());
    expect(screen.getByLabelText("MAC address")).toHaveValue("020000000002");
    expect(document.body.textContent).not.toContain(TOKEN);
    mocks.detect.mockResolvedValue(pc.mac);
    fireEvent.click(screen.getByText("Detect"));
    await waitFor(() =>
      expect(screen.getByLabelText("MAC address")).toHaveValue(pc.mac),
    );
    expect(mocks.begin).toHaveBeenCalledTimes(1);
  });
  it.each([
    "Pairing code is incorrect or the PC could not be verified.",
    "Pairing code expired or closed. Generate a new code.",
    "PC is unreachable.",
    "Host identity changed. Generate a new pairing code.",
    "PC is unreachable. Windows may retain this pairing.",
  ])("shows the safe pairing failure: %s", async (message) => {
    mocks.begin.mockRejectedValue(backendError(message));
    render(<SettingsEditorPage />);
    await connect();
    expect(await screen.findByRole("alert")).toHaveTextContent(message);
    expect(mocks.save).not.toHaveBeenCalled();
  });
  it("asks the PC to show a code before asking for it", async () => {
    render(<SettingsEditorPage />);
    fireEvent.change(screen.getByLabelText("IP address"), {
      target: { value: pc.host },
    });
    await openCodeModal("Next");
    expect(mocks.requestCode).toHaveBeenCalledWith(pc.host, "47991");
    codeModal();
    expect(
      screen.getByText("Enter the 8-digit code shown on your PC."),
    ).toBeInTheDocument();
  });
  it("falls back to a settings message when Decky gives no text", async () => {
    mocks.requestCode.mockRejectedValue(new Error("transport detail"));
    render(<SettingsEditorPage />);
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not update PC settings.",
    );
  });
  it("shows why the PC could not show a code and asks for none", async () => {
    mocks.requestCode.mockRejectedValue(backendError("PC is unreachable."));
    render(<SettingsEditorPage />);
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "PC is unreachable.",
    );
    expect(mocks.modal).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Next" })).toBeEnabled();
  });
  it("shows a generic message for unknown failures and rejects a short code before contacting the PC", async () => {
    render(<SettingsEditorPage />);
    await openCodeModal("Next");
    const modal = codeModal();
    fireEvent.change(screen.getByLabelText("Pairing code"), {
      target: { value: "12x٣456" },
    });
    expect(screen.getByLabelText("Pairing code")).toHaveValue("12456");
    expect(screen.getByRole("button", { name: "Pair" })).toBeDisabled();
    modal.unmount();
    expect(mocks.begin).not.toHaveBeenCalled();
    mocks.begin.mockRejectedValue(new Error(TOKEN));
    await connect();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not update PC settings",
    );
    expect(document.body.textContent).not.toContain(TOKEN);
  });
  it("cancels pending pairing on exit and ignores its late result", async () => {
    const pending = deferred<{
      draft_id: string;
      name: string;
      mac: string;
      mac_error: null;
    }>();
    mocks.begin.mockReturnValue(pending.promise);
    const editor = render(<SettingsEditorPage />);
    await connect();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(mocks.cancel).toHaveBeenCalledWith(mocks.begin.mock.calls[0][0]);
    expect(mocks.navigateBack).toHaveBeenCalledTimes(1);
    editor.unmount();
    await act(async () =>
      pending.resolve({
        draft_id: mocks.begin.mock.calls[0][0],
        name: "Late",
        mac: pc.mac,
        mac_error: null,
      }),
    );
    expect(mocks.save).not.toHaveBeenCalled();
    expect(mocks.navigateBack).toHaveBeenCalledTimes(1);
  });
  it("keeps a paired draft for field correction without re-pairing", async () => {
    mocks.save
      .mockRejectedValueOnce(
        backendError("Enter a MAC address with six hexadecimal pairs."),
      )
      .mockResolvedValue(pc);
    render(<SettingsEditorPage />);
    await pair();
    fireEvent.click(screen.getByRole("button", { name: "Save PC" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("MAC");
    fireEvent.change(screen.getByLabelText("MAC address"), {
      target: { value: "020000000002" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save PC" }));
    await waitFor(() => expect(mocks.navigateBack).toHaveBeenCalledTimes(1));
    expect(mocks.begin).toHaveBeenCalledTimes(1);
    expect(mocks.save.mock.calls[1][0]).toBe(mocks.save.mock.calls[0][0]);
  });
  it("edits an existing PC without replacing credentials", async () => {
    await existing();
    fireEvent.change(screen.getByLabelText("PC name"), {
      target: { value: "Renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(mocks.navigateBack).toHaveBeenCalledTimes(1));
    expect(mocks.update).toHaveBeenCalledWith(pc.id, {
      name: "Renamed",
      host: pc.host,
      port: "47991",
      mac: pc.mac,
    });
    expect(mocks.begin).not.toHaveBeenCalled();
  });
  it("orders the editor fields name, IP address, port, MAC address", async () => {
    await existing();
    const fields = ["PC name", "IP address", "Port", "MAC address"].map(
      (label) => screen.getByLabelText(label),
    );
    for (const [earlier, later] of fields
      .slice(0, -1)
      .map((f, i) => [f, fields[i + 1]]))
      expect(
        earlier.compareDocumentPosition(later) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
  });
  it("re-pairs an existing PC while preserving its chosen name", async () => {
    await existing();
    await openCodeModal("Pair again");
    expect(mocks.requestCode).toHaveBeenCalledWith(pc.host, "47991");
    enterCode();
    await waitFor(() => expect(mocks.begin).toHaveBeenCalled());
    // * The new credential belongs to the address it paired at.
    await waitFor(() =>
      expect(screen.getByLabelText("IP address")).toBeDisabled(),
    );
    expect(screen.getByLabelText("IP address")).toHaveValue(pc.host);
    expect(screen.getByLabelText("Port")).toBeDisabled();
    expect(mocks.begin.mock.calls[0][4]).toBe(pc.id);
    expect(screen.getByLabelText("PC name")).toHaveValue(pc.name);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(mocks.save).toHaveBeenCalled());
    expect(mocks.update).not.toHaveBeenCalled();
  });
  it("keeps the saved MAC when re-pairing cannot detect it", async () => {
    mocks.begin.mockImplementation(async (draft_id: string) => ({
      draft_id,
      name: "Fixture PC",
      mac: null,
      mac_error: "MAC detection failed. Enter the MAC address manually.",
    }));
    await existing();
    await openCodeModal("Pair again");
    enterCode();
    await waitFor(() => expect(mocks.begin).toHaveBeenCalled());
    await waitFor(() =>
      expect(screen.getByLabelText("IP address")).toBeDisabled(),
    );
    expect(screen.getByLabelText("MAC address")).toHaveValue(pc.mac);
  });
  it("does not cancel a pairing that was saved", async () => {
    render(<SettingsEditorPage />);
    await pair();
    fireEvent.click(screen.getByRole("button", { name: "Save PC" }));
    await waitFor(() => expect(mocks.navigateBack).toHaveBeenCalledTimes(1));
    expect(mocks.cancel).not.toHaveBeenCalled();
  });
  it("requires confirmation naming the PC before deleting it", async () => {
    await existing();
    fireEvent.click(screen.getByRole("button", { name: `Delete ${pc.name}` }));
    const first = renderLastModal();
    expect(
      screen.getByRole("heading", { name: `Delete ${pc.name}?` }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Keep PC" }));
    expect(mocks.remove).not.toHaveBeenCalled();
    first.unmount();
    fireEvent.click(screen.getByRole("button", { name: `Delete ${pc.name}` }));
    const secondElement = mocks.modal.mock.calls[1][0] as ReactElement<{
      closeModal?: () => void;
    }>;
    // noinspection TypeScriptValidateTypes
    render(
      cloneElement(secondElement, {
        closeModal: mocks.modal.mock.results[1].value.Close,
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(mocks.remove).toHaveBeenCalledWith(pc.id));
    await waitFor(() => expect(mocks.navigateBack).toHaveBeenCalled());
    expect(mocks.toast).not.toHaveBeenCalled();
  });
  it("says where to finish when the PC could not forget this device", async () => {
    mocks.remove.mockResolvedValue({ unpaired: false });
    await existing();
    fireEvent.click(screen.getByRole("button", { name: `Delete ${pc.name}` }));
    renderLastModal();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(mocks.toast).toHaveBeenCalledTimes(1));
    expect(mocks.toast.mock.calls[0]?.[0].body).toBe(
      `${pc.name} was unreachable. Remove this device in My Rig on Windows.`,
    );
  });
  it("retargets an open editor when a notification names another PC", async () => {
    const other = {
      ...pc,
      id: "22222222-2222-4222-8222-222222222222",
      name: "Office PC",
      host: "192.168.1.41",
    };
    mocks.settings.mockResolvedValue([pc, other]);
    EditorNavigation.open(pc.id);
    render(<SettingsEditorPage />);
    expect(await screen.findByDisplayValue(pc.name)).toBeInTheDocument();
    powerService.start();
    await emitPowerSnapshot([
      {
        id: other.id,
        name: other.name,
        state: "online",
        issue: "authentication",
      },
    ]);
    await act(async () => mocks.toast.mock.calls[0]?.[0].onClick());
    await waitFor(() =>
      expect(screen.getByLabelText("PC name")).toHaveValue("Office PC"),
    );
  });
  it("clears the previous PC's MAC when a new pairing cannot detect its address", async () => {
    render(<SettingsEditorPage />);
    await pair();
    expect(screen.getByLabelText("MAC address")).toHaveValue(pc.mac);
    fireEvent.click(screen.getByRole("button", { name: "Pair again" }));
    mocks.begin.mockImplementation(async (draft_id: string) => ({
      draft_id,
      name: "Office PC",
      mac: null,
      mac_error: "MAC detection failed. Enter the MAC address manually.",
    }));
    await connect("192.168.1.41", "87654321");
    await screen.findByLabelText("MAC address");
    expect(screen.getByLabelText("MAC address")).toHaveValue("");
    expect(screen.getByRole("alert")).toHaveTextContent("MAC detection failed");
    expect(mocks.save).not.toHaveBeenCalled();
  });
  it.each(["paired", "edit", "delete"])(
    "disables Cancel while a %s write must finish",
    async (kind) => {
      const pending = deferred<typeof pc>();
      if (kind === "paired") {
        mocks.save.mockReturnValue(pending.promise);
        render(<SettingsEditorPage />);
        await pair();
        fireEvent.click(screen.getByRole("button", { name: "Save PC" }));
      } else {
        await existing();
        if (kind === "edit") {
          mocks.update.mockReturnValue(pending.promise);
          fireEvent.click(screen.getByRole("button", { name: "Save" }));
        } else {
          mocks.remove.mockReturnValue(pending.promise);
          fireEvent.click(
            screen.getByRole("button", { name: `Delete ${pc.name}` }),
          );
          renderLastModal();
          fireEvent.click(screen.getByRole("button", { name: "Delete" }));
        }
      }
      expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
      fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
      expect(mocks.navigateBack).not.toHaveBeenCalled();
      await act(async () => pending.resolve(pc));
      await waitFor(() => expect(mocks.navigateBack).toHaveBeenCalledTimes(1));
    },
  );
});
