import { definePlugin, routerHook } from "@decky/api";
import { FaPowerOff } from "react-icons/fa";
import { powerService } from "./power/power.service";
import { PRODUCT_NAME } from "./shared/product.messages";
import { PowerPanel } from "./power/power.panel";
import { SettingsPage } from "./settings/settings.page";
import { SettingsEditorPage } from "./settings/settings-editor.page";
import { SETTINGS_ROUTE, SETTINGS_EDITOR_ROUTE } from "./shared/routes.consts";
import { TitleView } from "./shared/title.view";
export default definePlugin(() => {
  powerService.start();
  routerHook.addRoute(SETTINGS_ROUTE, SettingsPage, { exact: true });
  routerHook.addRoute(SETTINGS_EDITOR_ROUTE, SettingsEditorPage, {
    exact: true,
  });
  return {
    name: PRODUCT_NAME,
    titleView: <TitleView />,
    content: <PowerPanel />,
    icon: <FaPowerOff />,
    onDismount() {
      powerService.stop();
      routerHook.removeRoute(SETTINGS_ROUTE);
      routerHook.removeRoute(SETTINGS_EDITOR_ROUTE);
    },
  };
});
