import {
  DialogButton,
  DialogButtonPrimary,
  DialogControlsSection,
  Field,
} from "@decky/ui";
import { useEffect, useState, useSyncExternalStore } from "react";
import { api } from "../shared/api.service";
import type { PcSettings } from "../shared/api.types";
import { AlertView } from "../shared/alert.view";
import { EditorNavigation } from "../shared/navigation.service";
import { ADD_PC, NO_PC, PRODUCT_NAME } from "../shared/product.messages";
import { powerService } from "../power/power.service";
import { EDIT, LOADING, SETTINGS_ERROR, editLabel } from "./settings.messages";
import { emptyStyle, buttonStyle } from "./settings.styles";
import { SettingsLayout } from "./settings-layout.view";
import { attend } from "../shared/attention.service";

/** The status text follows the power store; the page around it does not. */
function PcStatus({ id, host }: { id: string; host: string }) {
  const status = useSyncExternalStore(powerService.subscribe, () =>
    powerService.statusOf(id),
  );
  return `${status} · ${host}`;
}

// noinspection HtmlUnknownAttribute
export function SettingsPage() {
  const [pcs, setPcs] = useState<PcSettings[] | null>(null);
  const [error, setError] = useState("");
  useEffect(() => attend(), []);
  useEffect(() => {
    let active = true;
    let latest = 0;
    const load = () => {
      const request = ++latest;
      api
        .fetchSettings()
        .then((next) => {
          if (!active || request !== latest) return;
          setPcs(next);
          setError("");
        })
        .catch(() => {
          if (active && request === latest) setError(SETTINGS_ERROR);
        });
    };
    // * Showing the page again is the retry: nothing else reloads the list.
    const onShow = () => {
      if (document.visibilityState === "visible") load();
    };
    EditorNavigation.reset();
    load();
    document.addEventListener("visibilitychange", onShow);
    return () => {
      active = false;
      document.removeEventListener("visibilitychange", onShow);
    };
  }, []);
  return (
    <SettingsLayout
      title={PRODUCT_NAME}
      action={
        <DialogButtonPrimary
          style={buttonStyle}
          onClick={() => EditorNavigation.open(null)}
        >
          {ADD_PC}
        </DialogButtonPrimary>
      }
    >
      {error && <AlertView message={error} />}
      {pcs === null && !error && <div style={emptyStyle}>{LOADING}</div>}
      {pcs?.length === 0 && <div style={emptyStyle}>{NO_PC}</div>}
      <DialogControlsSection>
        {pcs?.map((pc) => {
          return (
            <Field
              key={pc.id}
              label={pc.name}
              description={<PcStatus id={pc.id} host={pc.host} />}
              inlineWrap="keep-inline"
              childrenContainerWidth="min"
            >
              <DialogButton
                aria-label={editLabel(pc.name)}
                style={buttonStyle}
                onClick={() => EditorNavigation.open(pc.id)}
              >
                {EDIT}
              </DialogButton>
            </Field>
          );
        })}
      </DialogControlsSection>
    </SettingsLayout>
  );
}
