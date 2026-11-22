import React from "react";
import { useTranslation } from "react-i18next";
import { Cloud, Cpu, Info } from "lucide-react";
import { SettingsGroup } from "../ui/SettingsGroup";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { useModelStore } from "../../stores/modelStore";
import type { ModelInfo } from "@/bindings";

/**
 * Unified Kothalipi Bangla & AI Intelligence settings:
 * 1. Keep English words in English when speaking Bangla (default ON)
 * 2. Translate Bangla speech to English (works with Gemini API or offline Whisper)
 * 3. App-Aware Tone & Formatting (Chat = casual, Email = formal, IDE = Vibe Coding)
 * 4. Context-Aware Vibe Coding Mode (Bangla/Banglish -> English developer prompts)
 * 5. Smart Polish (Gemini punctuation & spelling cleanup)
 */
export const BanglaSpeechSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const { currentModel, models } = useModelStore();

  const keys = getSetting("cloud_api_keys") ?? {};
  const hasGeminiKey = Boolean(keys.gemini);
  const mode = getSetting("listen_mode") ?? "offline";

  const banglaMixedEnglish = getSetting("bangla_mixed_english") ?? true;
  const translateToEnglish = getSetting("translate_to_english") ?? false;
  const appAwareTone = getSetting("app_aware_tone") ?? true;
  const vibeCodingMode = getSetting("vibe_coding_mode") ?? false;
  const smartPolish = getSetting("smart_polish") ?? false;

  const currentModelInfo = models.find((m: ModelInfo) => m.id === currentModel);
  const localSupportsTranslation =
    currentModelInfo?.supports_translation ?? false;

  return (
    <SettingsGroup title={t("settings.kothalipi.banglaSpeech.title")}>
      {/* 1. Keep English words in English inside Bangla dictation */}
      <ToggleSwitch
        checked={banglaMixedEnglish}
        onChange={(enabled) => updateSetting("bangla_mixed_english", enabled)}
        isUpdating={isUpdating("bangla_mixed_english")}
        label={t("settings.kothalipi.banglaSpeech.mixedEnglish.label")}
        description={t(
          "settings.kothalipi.banglaSpeech.mixedEnglish.description",
        )}
        descriptionMode="tooltip"
        grouped={true}
      />

      {/* 2. Translate Bangla speech directly to English */}
      <ToggleSwitch
        checked={translateToEnglish}
        onChange={(enabled) => updateSetting("translate_to_english", enabled)}
        isUpdating={isUpdating("translate_to_english")}
        label={t("settings.advanced.translateToEnglish.label")}
        description={t("settings.advanced.translateToEnglish.description")}
        descriptionMode="tooltip"
        grouped={true}
      />

      {/* Clear engine status banner for Translation */}
      <div className="mx-4 my-2 flex items-start gap-2 rounded-lg border border-line/60 bg-surface/40 px-3 py-2 text-xs text-text/75">
        {hasGeminiKey ? (
          <>
            <Cloud
              size={14}
              className="mt-0.5 shrink-0 text-sky-600 dark:text-sky-400"
              aria-hidden="true"
            />
            <span>
              {t("settings.kothalipi.banglaSpeech.translateHint.geminiReady")}
            </span>
          </>
        ) : localSupportsTranslation && currentModelInfo ? (
          <>
            <Cpu
              size={14}
              className="mt-0.5 shrink-0 text-emerald-600 dark:text-emerald-400"
              aria-hidden="true"
            />
            <span>
              {t("settings.kothalipi.banglaSpeech.translateHint.localReady", {
                model: currentModelInfo.name,
              })}
            </span>
          </>
        ) : (
          <>
            <Info
              size={14}
              className="mt-0.5 shrink-0 text-amber-600 dark:text-amber-400"
              aria-hidden="true"
            />
            <span>
              {t(
                mode === "cloud"
                  ? "settings.kothalipi.banglaSpeech.translateHint.needGemini"
                  : "settings.kothalipi.banglaSpeech.translateHint.needEngine",
              )}
            </span>
          </>
        )}
      </div>

      {/* 3. App-Aware Tone & Formatting */}
      <ToggleSwitch
        checked={appAwareTone}
        onChange={(enabled) => updateSetting("app_aware_tone", enabled)}
        isUpdating={isUpdating("app_aware_tone")}
        disabled={!hasGeminiKey}
        label={t("settings.kothalipi.banglaSpeech.appAwareTone.label")}
        description={t(
          hasGeminiKey
            ? "settings.kothalipi.banglaSpeech.appAwareTone.description"
            : "settings.kothalipi.cloud.polish.needKey",
        )}
        descriptionMode="tooltip"
        grouped={true}
      />

      {/* 4. Vibe Coding Mode */}
      <ToggleSwitch
        checked={vibeCodingMode}
        onChange={(enabled) => updateSetting("vibe_coding_mode", enabled)}
        isUpdating={isUpdating("vibe_coding_mode")}
        disabled={!hasGeminiKey}
        label={t("settings.kothalipi.banglaSpeech.vibeCoding.label")}
        description={t(
          hasGeminiKey
            ? "settings.kothalipi.banglaSpeech.vibeCoding.description"
            : "settings.kothalipi.cloud.polish.needKey",
        )}
        descriptionMode="tooltip"
        grouped={true}
      />

      {/* 5. Smart Polish */}
      <ToggleSwitch
        checked={smartPolish}
        onChange={(enabled) => updateSetting("smart_polish", enabled)}
        isUpdating={isUpdating("smart_polish")}
        disabled={!hasGeminiKey}
        label={t("settings.kothalipi.cloud.polish.label")}
        description={t(
          hasGeminiKey
            ? "settings.kothalipi.cloud.polish.description"
            : "settings.kothalipi.cloud.polish.needKey",
        )}
        descriptionMode="tooltip"
        grouped={true}
      />
    </SettingsGroup>
  );
};
