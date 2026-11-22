import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// noinspection TypeScriptUnresolvedReference
vi.mock(
  "@decky/api",
  async () => (await import("./shared/decky.fixtures")).deckyApi,
);
// noinspection TypeScriptUnresolvedReference
vi.mock(
  "@decky/ui",
  async () => (await import("./shared/decky.fixtures")).deckyUi,
);

import plugin from "./index";
import { powerService } from "./power/power.service";
import { SettingsPage } from "./settings/settings.page";
import { SettingsEditorPage } from "./settings/settings-editor.page";
import { EditorNavigation } from "./shared/navigation.service";
import { SETTINGS_ROUTE, SETTINGS_EDITOR_ROUTE } from "./shared/routes.consts";
import {
  mocks,
  releaseSnapshots,
  resetDeckyMocks,
} from "./shared/decky.fixtures";

afterEach(async () => {
  powerService.stop();
  await releaseSnapshots();
});
beforeEach(() => {
  EditorNavigation.reset();
  resetDeckyMocks();
});

describe("plugin registration and API", () => {
  it("binds every backend RPC and owns its settings routes", () => {
    expect(new Set(mocks.routes)).toEqual(
      new Set([
        "settings",
        "begin_pairing",
        "request_pairing_code",
        "save_pairing",
        "cancel_pairing",
        "detect_mac",
        "update_pc",
        "delete_pc",
        "start",
        "stop",
        "watch_snapshot",
        "set_attention",
      ]),
    );
    const definition = plugin() as {
      content: ReactElement;
      titleView: ReactElement;
      onDismount(): void;
    };
    expect(mocks.addRoute).toHaveBeenCalledWith(SETTINGS_ROUTE, SettingsPage, {
      exact: true,
    });
    expect(mocks.addRoute).toHaveBeenCalledWith(
      SETTINGS_EDITOR_ROUTE,
      SettingsEditorPage,
      { exact: true },
    );
    render(definition.titleView);
    expect(
      screen
        .getByRole("button", { name: "Open Decky My Rig settings" })
        .querySelector("svg"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Open Decky My Rig settings" }),
    ).not.toHaveTextContent("Settings");
    fireEvent.click(
      screen.getByRole("button", { name: "Open Decky My Rig settings" }),
    );
    expect(mocks.navigate).toHaveBeenCalledWith(SETTINGS_ROUTE);
    definition.onDismount();
    expect(mocks.removeRoute).toHaveBeenCalledWith(SETTINGS_ROUTE);
    expect(mocks.removeRoute).toHaveBeenCalledWith(SETTINGS_EDITOR_ROUTE);
  });
});
