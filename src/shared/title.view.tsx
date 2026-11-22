import { DialogButton, Focusable, Navigation, staticClasses } from "@decky/ui";
import { FaCog } from "react-icons/fa";
import { PRODUCT_NAME, SETTINGS_LABEL } from "./product.messages";
import { SETTINGS_ROUTE } from "./routes.consts";
import { titleButtonStyle, titleStyle, titleTextStyle } from "./title.styles";

export function TitleView() {
  return (
    <Focusable className={staticClasses.Title} style={titleStyle}>
      <div style={titleTextStyle}>{PRODUCT_NAME}</div>
      <DialogButton
        aria-label={SETTINGS_LABEL}
        style={titleButtonStyle}
        onClick={() => {
          Navigation.CloseSideMenus();
          Navigation.Navigate(SETTINGS_ROUTE);
        }}
      >
        <FaCog size="0.9em" />
      </DialogButton>
    </Focusable>
  );
}
