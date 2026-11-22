import {
  DialogButton,
  Field,
  PanelSection,
  PanelSectionRow,
  showModal,
} from "@decky/ui";
import { useEffect, useSyncExternalStore } from "react";
import { AlertView } from "../shared/alert.view";
import { attend } from "../shared/attention.service";
import { EditorNavigation } from "../shared/navigation.service";
import { ADD_PC, NO_PC } from "../shared/product.messages";
import { POWER_LABELS, POWER_STATUS, actionLabel } from "./power.messages";
import { StopConfirmation } from "./stop-confirmation.view";
import { powerService } from "./power.service";
import { actionStyle, rowActionStyle } from "./power.styles";
import { stateAction, statusMessage } from "./power.utils";

// noinspection HtmlUnknownAttribute
export function PowerPanel() {
  const { pcs, loaded, available, pending, error } = useSyncExternalStore(
    powerService.subscribe,
    powerService.getSnapshot,
  );
  useEffect(() => attend(), []);

  return (
    <PanelSection>
      {error && (
        <PanelSectionRow>
          <AlertView message={error} />
        </PanelSectionRow>
      )}
      {!loaded && (
        <PanelSectionRow>
          <Field focusable={false} label={POWER_STATUS.checking} />
        </PanelSectionRow>
      )}
      {loaded && pcs.length === 0 && (
        <PanelSectionRow>
          <Field
            label={NO_PC}
            inlineWrap="keep-inline"
            childrenContainerWidth="min"
          >
            <DialogButton
              style={actionStyle}
              onClick={() => EditorNavigation.open(null)}
            >
              {ADD_PC}
            </DialogButton>
          </Field>
        </PanelSectionRow>
      )}
      {pcs.map((pc) => {
        const action = stateAction(pc, pending.get(pc.id));
        return (
          <PanelSectionRow key={pc.id}>
            <Field
              label={pc.name}
              description={statusMessage(pc)}
              inlineWrap="keep-inline"
              childrenContainerWidth="min"
            >
              <DialogButton
                aria-label={actionLabel(action.kind, pc.name)}
                // ? Fix opens the settings, which still work when power status
                //   doesn't.
                disabled={
                  !action.action || (!available && action.action !== "fix")
                }
                style={rowActionStyle}
                onClick={() => {
                  if (action.action === "fix") EditorNavigation.open(pc.id);
                  else if (action.action === "stop")
                    showModal(<StopConfirmation pc={pc} />);
                  else void powerService.act(pc.id, "start");
                }}
              >
                {POWER_LABELS[action.kind]}
              </DialogButton>
            </Field>
          </PanelSectionRow>
        );
      })}
    </PanelSection>
  );
}
