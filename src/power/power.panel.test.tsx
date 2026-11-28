import "@testing-library/jest-dom/vitest";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
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

import { powerService } from "./power.service";
import { PowerPanel } from "./power.panel";
import { TitleView } from "../shared/title.view";
import { EditorNavigation } from "../shared/navigation.service";
import { SETTINGS_ROUTE, SETTINGS_EDITOR_ROUTE } from "../shared/routes.consts";
import {
  deferred,
  emitPowerSnapshot,
  mocks,
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

describe("PowerPanel", () => {
  beforeEach(() => powerService.start());

  it("tells the agent while it is on screen", async () => {
    await emitPowerSnapshot([]);
    const { unmount } = render(<PowerPanel />);
    await waitFor(() => expect(mocks.attention).toHaveBeenLastCalledWith(true));
    unmount();
    await waitFor(() =>
      expect(mocks.attention).toHaveBeenLastCalledWith(false),
    );
  });

  it("offers Add PC in an empty QAM beside the header settings button", async () => {
    await emitPowerSnapshot([]);
    render(
      <>
        <TitleView />
        <PowerPanel />
      </>,
    );
    expect(screen.getByText("No PC")).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Open Decky My Rig settings" }),
    );
    expect(mocks.navigate).toHaveBeenCalledWith(SETTINGS_ROUTE);
    fireEvent.click(screen.getByText("Add PC"));
    expect(mocks.navigate).toHaveBeenCalledWith(SETTINGS_EDITOR_ROUTE);
    expect(EditorNavigation.getSnapshot()).toBeNull();
  });

  it("shows each state, disables unavailable actions, and keeps rows minimal", async () => {
    await emitPowerSnapshot([
      { id: "a", name: "Offline PC", state: "offline" },
      { id: "b", name: "Checking PC", state: "checking" },
      { id: "c", name: "Starting PC", state: "starting" },
      { id: "d", name: "Stopping PC", state: "stopping", issue: "identity" },
      { id: "e", name: "Online PC", state: "online" },
    ]);
    render(<PowerPanel />);
    expect(
      screen.getByRole("button", { name: "Start Offline PC" }),
    ).toBeEnabled();
    for (const name of [
      "Checking Checking PC",
      "Starting Starting PC",
      "Stopping Stopping PC",
    ])
      expect(screen.getByRole("button", { name })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Stop Online PC" }),
    ).toBeEnabled();
    expect(document.querySelectorAll("[data-field]")).toHaveLength(5);
    expect(screen.getByText("Offline", { exact: true })).toBeInTheDocument();
  });

  it("offers Fix for online PCs with issues and keeps Start for offline ones", async () => {
    await emitPowerSnapshot([
      {
        id: "a",
        name: "Credential PC",
        state: "online",
        issue: "authentication",
      },
      {
        id: "b",
        name: "Old host PC",
        state: "offline",
        issue: "compatibility",
      },
    ]);
    expect(mocks.toast).toHaveBeenCalledTimes(2);
    render(<PowerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Fix Credential PC" }));
    expect(
      screen.getByRole("button", { name: "Start Old host PC" }),
    ).toBeEnabled();
    expect(mocks.navigate).toHaveBeenCalledWith(SETTINGS_EDITOR_ROUTE);
    expect(mocks.stop).not.toHaveBeenCalled();
  });

  it("keeps a Start reservation across QAM mounts and follows backend transitions", async () => {
    const started = deferred<void>();
    mocks.start.mockReturnValue(started.promise);
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "offline" }]);
    const view = render(<PowerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Start Office PC" }));
    view.unmount();
    render(<PowerPanel />);
    expect(
      screen.getByRole("button", { name: "Starting Office PC" }),
    ).toBeDisabled();
    await emitPowerSnapshot([
      { id: "a", name: "Office PC", state: "starting" },
    ]);
    await act(async () => started.resolve());
    expect(
      screen.getByRole("button", { name: "Starting Office PC" }),
    ).toBeDisabled();
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "online" }]);
    expect(
      screen.getByRole("button", { name: "Stop Office PC" }),
    ).toBeEnabled();
    expect(mocks.start).toHaveBeenCalledTimes(1);
  });

  it("reserves and sends Stop only after naming the PC in a confirmation", async () => {
    const stopped = deferred<boolean>();
    mocks.stop.mockReturnValue(stopped.promise);
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "online" }]);
    render(<PowerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Stop Office PC" }));
    expect(mocks.stop).not.toHaveBeenCalled();
    renderLastModal();
    expect(
      screen.getByRole("heading", { name: "Stop Office PC?" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(mocks.stop).toHaveBeenCalledWith("a");
    expect(
      screen.getByRole("button", { name: "Stopping Office PC" }),
    ).toBeDisabled();
    await emitPowerSnapshot([
      { id: "a", name: "Office PC", state: "stopping" },
    ]);
    await act(async () => stopped.resolve(false));
    expect(
      screen.getByRole("button", { name: "Stopping Office PC" }),
    ).toBeDisabled();
  });

  it("opens one Stop confirmation for a repeated press", async () => {
    // ! A second press while the question is open must not stack a second one, or two confirmations
    // ! stay on screen after one is answered.
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "online" }]);
    render(<PowerPanel />);
    const stop = screen.getByRole("button", { name: "Stop Office PC" });
    fireEvent.click(stop);
    fireEvent.click(stop);
    expect(mocks.modal).toHaveBeenCalledTimes(1);
    // ? Rendering the question lets cleanup close it, as it does on screen.
    renderLastModal();
  });

  it("does not send Stop when its confirmation is canceled", async () => {
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "online" }]);
    render(<PowerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Stop Office PC" }));
    renderLastModal();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(mocks.stop).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: "Stop Office PC" }),
    ).toBeEnabled();
  });

  it("shows the test shutdown notice after the panel unmounts and stays Online", async () => {
    const stopped = deferred<boolean>();
    mocks.stop.mockReturnValue(stopped.promise);
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "online" }]);
    const view = render(<PowerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Stop Office PC" }));
    const modal = renderLastModal();
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    modal.unmount();
    view.unmount();
    await emitPowerSnapshot(
      [{ id: "a", name: "Office PC", state: "online" }],
      1,
      "Office PC",
    );
    await act(async () => stopped.resolve(true));
    expect(mocks.toast).toHaveBeenCalledWith(
      expect.objectContaining({ body: "Test shutdown received." }),
    );
    render(<PowerPanel />);
    expect(
      screen.getByRole("button", { name: "Stop Office PC" }),
    ).toBeEnabled();
  });

  it("shows the backend's reason for a failed action", async () => {
    mocks.start.mockRejectedValue(backendError("PC could not be started."));
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "offline" }]);
    render(<PowerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Start Office PC" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Office PC: PC could not be started.",
    );
  });
  it("shows a generic message for backend errors and clears it on the next good snapshot", async () => {
    mocks.start.mockRejectedValue(new Error("private backend detail"));
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "offline" }]);
    render(<PowerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Start Office PC" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Office PC");
    expect(document.body.textContent).not.toContain("private backend detail");
    await emitPowerSnapshot([{ id: "a", name: "Office PC", state: "offline" }]);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
