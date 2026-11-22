import {
  DialogButton,
  DialogButtonPrimary,
  DialogControlsSection,
  Field,
} from "@decky/ui";
import { useEffect, useState, useSyncExternalStore } from "react";
import { fetchSettings } from "../shared/api.service";
import type { PcSettings } from "../shared/api.types";
import { AlertView } from "../shared/alert.view";
import { EditorNavigation } from "../shared/navigation.service";
import { ADD_PC, NO_PC, PRODUCT_NAME } from "../shared/product.messages";
import { powerService } from "../power/power.service";
import { EDIT, LOADING, SETTINGS_ERROR, editLabel } from "./settings.messages";
import { emptyStyle, buttonStyle } from "./settings.styles";
import { SettingsLayout } from "./settings-layout.view";
import { attend } from "../shared/attention.service";

// noinspection HtmlUnknownAttribute
export function SettingsPage() {
  const [pcs, setPcs] = useState<PcSettings[] | null>(null);
  const [error, setError] = useState("");
  // - Re-render when power status changes.
  useSyncExternalStore(powerService.subscribe, powerService.getSnapshot);
  useEffect(() => attend(), []);
  useEffect(() => {
    let active = true;
    EditorNavigation.reset();
    void fetchSettings()
      .then((next) => {
        if (active) setPcs(next);
      })
      .catch(() => {
        if (active) setError(SETTINGS_ERROR);
      });
    return () => {
      active = false;
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
              description={`${powerService.statusOf(pc.id)} · ${pc.host}`}
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
