import React, { useEffect, useLayoutEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { commands, type ListenMode } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import {
  STEP_ORDER,
  clearOnboardingProgress,
  useOnboardingStore,
  type OnboardingStep,
} from "./onboardingStore";
import { usePermissions } from "./usePermissions";
import { Permissions } from "./screens/Permissions";
import { TryIt } from "./screens/TryIt";
import { CloudKeys, Keys, Listen, Voices } from "./screens/KothalipiSetup";
import { useOsType } from "@/hooks/useOsType";
import { useOnboardingModels } from "./useOnboardingModels";
import { useModelStore } from "@/stores/modelStore";
import { moveScreens } from "./screenTransitions";
import "./screenTransitions.css";

interface OnboardingFlowProps {
  /** Debug preview: inert (no prompts, downloads or setting writes). */
  preview?: boolean;
  /** The screen a preview opens on. */
  previewStep?: OnboardingStep;
  /** Called after the last screen, to show Settings. */
  onComplete: () => void;
}

/**
 * Kothalipi first-run onboarding: permissions, offline or cloud, cloud keys,
 * voice models, language keys, then a first dictation. Progress persists, so
 * a relaunch or quit resumes.
 */
export const OnboardingFlow: React.FC<OnboardingFlowProps> = ({
  preview = false,
  previewStep,
  onComplete,
}) => {
  const { i18n } = useTranslation();
  const store = useOnboardingStore();
  const { settings, updateSetting } = useSettings();
  const permissions = usePermissions(preview);
  const osType = useOsType();
  const models = useOnboardingModels(store.languages);
  const banglaReady = useModelStore((s) =>
    s.models.some(
      (m) => m.is_downloaded && m.supported_languages.includes("bn"),
    ),
  );
  const listenMode: ListenMode = settings?.listen_mode ?? "offline";
  const cloudKeys = settings?.cloud_api_keys ?? {};
  const cloudReady =
    listenMode === "cloud" && Boolean(cloudKeys.gemini || cloudKeys.groq || cloudKeys.openai);

  // Kothalipi speaks Bangla and English.
  const languagesSaved = useRef(false);
  useEffect(() => {
    if (preview || !settings || languagesSaved.current) return;
    languagesSaved.current = true;
    updateSetting("spoken_languages", ["bn", "en"]);
  }, [preview, settings]);

  // Try it teaches hold and tap, which only hold_or_toggle supports.
  const activationSet = useRef(false);
  const activation = settings?.shortcut_activation;
  useEffect(() => {
    if (preview || !activation || activationSet.current) return;
    activationSet.current = true;
    if (activation !== "hold_or_toggle")
      updateSetting("shortcut_activation", "hold_or_toggle");
  }, [preview, activation]);

  // Before first paint so a preview never shows the saved step.
  useLayoutEffect(() => {
    store.reset(preview);
    if (preview && previewStep) store.set({ step: previewStep });
    return () => {
      if (preview) store.reset(false);
    };
    // Once per mount.
  }, []);

  // Restore the UI language a preview may have changed.
  useEffect(() => {
    if (!preview) return;
    const before = i18n.language;
    return () => {
      if (i18n.language !== before) i18n.changeLanguage(before);
    };
  }, [preview]);

  if (!permissions.checked) return null;

  const stepsFor = (mode: ListenMode) =>
    STEP_ORDER.filter(
      (s) =>
        (s !== "permissions" || permissions.needed || store.welcomeBack) &&
        (s !== "cloud" || mode === "cloud"),
    );
  const steps = stepsFor(listenMode);
  const step = steps.includes(store.step) ? store.step : steps[0];
  const progress = { index: steps.indexOf(step), count: steps.length };

  // Reaching returnTo ends the detour.
  const go = (to: OnboardingStep) =>
    store.set(
      to === store.returnTo ? { step: to, returnTo: null } : { step: to },
    );
  const next = (list = steps) => {
    const i = list.indexOf(step);
    if (store.returnTo) go(store.returnTo);
    else if (i < list.length - 1) go(list[i + 1]);
    else finish();
  };
  const back = () => {
    const i = steps.indexOf(step);
    if (i > 0) go(steps[i - 1]);
  };

  const finish = async () => {
    // Saved only now: each login item change posts a macOS notification.
    if (!preview) {
      await updateSetting("autostart_enabled", store.atLogin);
      await commands.completeOnboarding();
    }
    onComplete();
    // After Settings replaces the flow, so the first screen doesn't flash.
    if (!preview) clearOnboardingProgress();
  };

  switch (step) {
    case "permissions":
      return (
        <Permissions
          permissions={permissions}
          progress={progress}
          onBack={progress.index > 0 ? back : undefined}
          onContinue={() => next()}
        />
      );
    case "listen":
      return (
        <Listen
          progress={progress}
          onBack={progress.index > 0 ? back : undefined}
          onContinue={async (mode) => {
            if (!preview) await updateSetting("listen_mode", mode);
            // The step list depends on the mode just chosen.
            next(stepsFor(mode));
          }}
        />
      );
    case "cloud":
      return (
        <CloudKeys
          progress={progress}
          onBack={back}
          onContinue={() => next()}
        />
      );
    case "voices":
      return (
        <Voices
          progress={progress}
          cloud={listenMode === "cloud"}
          onBack={back}
          onContinue={() => next()}
        />
      );
    case "keys":
      return (
        <Keys
          progress={progress}
          isMac={osType === "macos"}
          onBack={back}
          onContinue={() => moveScreens("in", () => next())}
        />
      );
    // Same element for both steps, so Try it stays mounted into All set.
    case "try":
    case "done":
    default:
      return (
        <TryIt
          key="try"
          progress={progress}
          os={osType}
          finished={step === "done"}
          model={models.catalogFor(settings?.selected_model ?? "")}
          ready={preview || Boolean(banglaReady) || cloudReady}
          download={null}
          onChangeModel={() => go("voices")}
          onEdit={(to) =>
            store.set({
              step: to === "model" ? "voices" : "keys",
              returnTo: "done",
            })
          }
          onBack={() => moveScreens("out", back)}
          onArrive={() => go("done")}
          onUnfinish={() => go("try")}
          onDone={finish}
        />
      );
  }
};
