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
import type { StopConfirmationProps } from "./power.types";
import { StopConfirmation } from "./stop-confirmation.view";
import { powerService } from "./power.service";
import { actionStyle, rowActionStyle } from "./power.styles";
import { stateAction, statusMessage } from "./power.utils";

// noinspection HtmlUnknownAttribute
// ! Only one Stop question may be open. A repeated press while it is on screen must not add
// ! another, or two stay open after one is answered.
let stopQuestionOpen = false;

function openStopConfirmation(pc: StopConfirmationProps["pc"]) {
  if (stopQuestionOpen) return;
  stopQuestionOpen = true;
  try {
    showModal(
      <StopConfirmation
        pc={pc}
        onDone={() => {
          stopQuestionOpen = false;
        }}
      />,
    );
  } catch (error) {
    // ? If the question never mounts, nothing resets the flag, so reset it here.
    stopQuestionOpen = false;
    throw error;
  }
}

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
                // ? Fix opens the settings, which still work when power status doesn't.
                disabled={
                  !action.action || (!available && action.action !== "fix")
                }
                style={rowActionStyle}
                onClick={() => {
                  if (action.action === "fix") EditorNavigation.open(pc.id);
                  else if (action.action === "stop") openStopConfirmation(pc);
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
