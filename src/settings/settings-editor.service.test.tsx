import "@testing-library/jest-dom/vitest";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import type { ReactElement } from "react";
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

import { SettingsEditorPage } from "./settings-editor.page";
import { EditorNavigation } from "../shared/navigation.service";
import {
  backendError,
  deferred,
  mocks,
  pc,
  releaseSnapshots,
  renderLastModal,
  resetDeckyMocks,
} from "../shared/decky.fixtures";

afterEach(async () => {
  await releaseSnapshots();
});
beforeEach(() => {
  EditorNavigation.reset();
  resetDeckyMocks();
});

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}
async function openCodeModal() {
  type("IP address", pc.host);
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  await waitFor(() => expect(mocks.modal).toHaveBeenCalledTimes(1));
}
function submitCode() {
  const modal = renderLastModal();
  type("Pairing code", "12345678");
  fireEvent.click(screen.getByRole("button", { name: "Pair" }));
  modal.unmount();
}
async function paired() {
  await openCodeModal();
  submitCode();
  await screen.findByLabelText("PC name");
}

describe("editor state read by callbacks that outlive a render", () => {
  it("pairs with the address and port as they are when the code is submitted", async () => {
    render(<SettingsEditorPage />);
    await openCodeModal();
    type("IP address", "10.0.0.9");
    type("Port", "48000");
    submitCode();
    await waitFor(() => expect(mocks.begin).toHaveBeenCalled());
    expect(mocks.begin.mock.calls[0].slice(1, 4)).toEqual([
      "10.0.0.9",
      "48000",
      "12345678",
    ]);
  });
});

describe("cancelling a pairing draft", () => {
  it("cancels a draft once when the editor is left during pairing", async () => {
    const pending = deferred<unknown>();
    mocks.begin.mockReturnValue(pending.promise);
    render(<SettingsEditorPage />);
    await openCodeModal();
    submitCode();
    await waitFor(() => expect(mocks.begin).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await act(async () => pending.resolve({ draft_id: "x", name: "Late" }));
    expect(mocks.cancel).toHaveBeenCalledTimes(1);
  });

  it("shows a failed cancel when pairing again", async () => {
    render(<SettingsEditorPage />);
    await paired();
    mocks.cancel.mockRejectedValue(backendError("PC is unreachable."));
    fireEvent.click(screen.getByRole("button", { name: "Pair again" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "PC is unreachable.",
    );
  });

  it("tells the user about a failed cancel when the editor is left", async () => {
    render(<SettingsEditorPage />);
    await paired();
    mocks.cancel.mockRejectedValue(backendError("PC is unreachable."));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(mocks.toast).toHaveBeenCalledTimes(1));
    expect(mocks.toast.mock.calls[0]?.[0].body).toBe("PC is unreachable.");
  });
});

describe("editor rendering", () => {
  it("re-renders only the field that was typed in", async () => {
    render(<SettingsEditorPage />);
    await paired();
    mocks.fieldRender.mockClear();
    type("PC name", "My PC");
    const rendered = new Set(
      mocks.fieldRender.mock.calls.map(
        ([label]) =>
          (label as ReactElement<{ children: string }>).props.children,
      ),
    );
    expect([...rendered]).toEqual(["PC name"]);
  });
});
