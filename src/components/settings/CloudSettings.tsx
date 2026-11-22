import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ArrowDown,
  ArrowUp,
  Check,
  Cpu,
  Cloud,
  Info,
  Sparkles,
} from "lucide-react";
import {
  commands,
  type CloudEngine,
  type CloudProvider,
  type ListenMode,
} from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { useModelStore } from "../../stores/modelStore";
import { Button } from "../ui/Button";
import { Dropdown } from "../ui/Dropdown";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";
import { SettingsGroup } from "../ui/SettingsGroup";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { CloudUsagePanel } from "./CloudUsagePanel";

const KEY_URLS: Record<CloudProvider, string> = {
  gemini: "https://aistudio.google.com/apikey",
  openai: "https://platform.openai.com/api-keys",
};

/** One provider: key (checked before saving), model, enabled, order. */
const EngineRow: React.FC<{
  engine: CloudEngine;
  index: number;
  count: number;
  hasKey: boolean;
  onChange: (engine: CloudEngine) => void;
  onMove: (delta: -1 | 1) => void;
}> = ({ engine, index, count, hasKey, onChange, onMove }) => {
  const { t } = useTranslation();
  const { refreshSettings } = useSettings();
  const [key, setKey] = useState("");
  const [status, setStatus] = useState<"idle" | "checking" | "saved" | "bad">(
    "idle",
  );
  const [error, setError] = useState("");
  const name = t(`settings.kothalipi.cloud.providers.${engine.provider}`);

  const save = async () => {
    setStatus("checking");
    const ok = await commands.checkCloudApiKey(engine.provider, key);
    if (ok.status === "error") {
      setError(ok.error);
      setStatus("bad");
      return;
    }
    await commands.changeCloudApiKeySetting(engine.provider, key);
    await refreshSettings();
    setKey("");
    setStatus("saved");
  };

  return (
    <SettingContainer
      title={`${index + 1}. ${name}`}
      description={t("settings.kothalipi.cloud.rowDescription")}
      descriptionMode="tooltip"
      grouped={true}
      layout="stacked"
    >
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <Input
            type="password"
            autoComplete="off"
            spellCheck={false}
            className="flex-1"
            placeholder={t(
              hasKey
                ? "settings.kothalipi.cloud.keySaved"
                : "settings.kothalipi.cloud.keyPlaceholder",
            )}
            aria-label={t("settings.kothalipi.cloud.keyLabel", { name })}
            value={key}
            onChange={(e) => {
              setKey(e.target.value);
              setStatus("idle");
            }}
          />
          <Button
            variant="secondary"
            size="sm"
            disabled={!key.trim() || status === "checking"}
            onClick={save}
          >
            {t("settings.kothalipi.cloud.save")}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => openUrl(KEY_URLS[engine.provider])}
          >
            {t("settings.kothalipi.cloud.getKey")}
          </Button>
        </div>
        {status === "saved" && (
          <span className="inline-flex items-center gap-1 text-sm text-green-600">
            <Check size={14} aria-hidden="true" />
            {t("settings.kothalipi.cloud.keyWorks")}
          </span>
        )}
        {status === "bad" && (
          <span className="text-sm text-red-500">{error}</span>
        )}
        <div className="flex items-center gap-2">
          <Input
            variant="compact"
            className="flex-1"
            aria-label={t("settings.kothalipi.cloud.modelLabel", { name })}
            value={engine.model}
            onChange={(e) => onChange({ ...engine, model: e.target.value })}
          />
          <label className="flex items-center gap-1 text-sm">
            <input
              type="checkbox"
              checked={engine.enabled}
              onChange={(e) =>
                onChange({ ...engine, enabled: e.target.checked })
              }
            />
            {t("settings.kothalipi.cloud.enabled")}
          </label>
          <Button
            variant="ghost"
            size="sm"
            disabled={index === 0}
            aria-label={t("settings.kothalipi.cloud.moveUp")}
            onClick={() => onMove(-1)}
          >
            <ArrowUp size={14} />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={index === count - 1}
            aria-label={t("settings.kothalipi.cloud.moveDown")}
            onClick={() => onMove(1)}
          >
            <ArrowDown size={14} />
          </Button>
        </div>
      </div>
    </SettingContainer>
  );
};

/** Kothalipi: offline or cloud, and the cloud engines in fallback order. */
export const CloudSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, refreshSettings, isUpdating } =
    useSettings();
  const { currentModel, models } = useModelStore();
  const mode: ListenMode = getSetting("listen_mode") ?? "offline";
  const localModelEnabled = getSetting("local_model_enabled") ?? true;
  const fallbackEnabled = getSetting("fallback_enabled") ?? true;
  const engines = getSetting("cloud_engines") ?? [];
  const keys = getSetting("cloud_api_keys") ?? {};

  const currentModelInfo = models.find((m) => m.id === currentModel);
  const enabledEnginesWithKey = engines.filter(
    (e) => e.enabled && Boolean(keys[e.provider]),
  );
  const activeCloudEngine = enabledEnginesWithKey[0];
  const cloudIsPrimary = mode === "cloud" || !localModelEnabled;

  // Build ordered priority chain for live visual preview
  const priorityChain: Array<{ label: string; active: boolean }> = [];
  const cloudItems = engines.map((e) => ({
    label: t(`settings.kothalipi.cloud.providers.${e.provider}`),
    active: e.enabled && Boolean(keys[e.provider]),
  }));
  const localItem = {
    label: currentModelInfo?.name || t("settings.kothalipi.mode.offline"),
    active: localModelEnabled && Boolean(currentModelInfo?.is_downloaded),
  };
  if (mode === "cloud") {
    priorityChain.push(...cloudItems);
    if (fallbackEnabled) priorityChain.push(localItem);
  } else {
    if (localModelEnabled) priorityChain.push(localItem);
    if (fallbackEnabled || !localModelEnabled)
      priorityChain.push(...cloudItems);
  }

  const postProcessEnabled = getSetting("post_process_enabled") ?? false;
  const postProcessPromptId = getSetting("post_process_selected_prompt_id");
  const postProcessPrompts = getSetting("post_process_prompts") ?? [];
  const activePrompt = postProcessPrompts.find(
    (p) => p.id === postProcessPromptId,
  );

  const saveEngines = async (next: CloudEngine[]) => {
    await commands.changeCloudEnginesSetting(next);
    await refreshSettings();
  };

  return (
    <SettingsGroup title={t("settings.kothalipi.title")}>
      <SettingContainer
        title={t("settings.kothalipi.mode.title")}
        description={t("settings.kothalipi.mode.description")}
        descriptionMode="tooltip"
        grouped={true}
      >
        <Dropdown
          options={[
            {
              value: "offline",
              label: t("settings.kothalipi.mode.offlineFirst"),
            },
            { value: "cloud", label: t("settings.kothalipi.mode.cloudFirst") },
          ]}
          selectedValue={mode}
          onSelect={(v) => updateSetting("listen_mode", v as ListenMode)}
          disabled={isUpdating("listen_mode")}
        />
      </SettingContainer>

      <ToggleSwitch
        checked={localModelEnabled}
        onChange={(enabled) => updateSetting("local_model_enabled", enabled)}
        isUpdating={isUpdating("local_model_enabled")}
        label={t("settings.kothalipi.engineControls.localModel.label")}
        description={t(
          "settings.kothalipi.engineControls.localModel.description",
        )}
        descriptionMode="tooltip"
        grouped={true}
      />

      <ToggleSwitch
        checked={fallbackEnabled}
        onChange={(enabled) => updateSetting("fallback_enabled", enabled)}
        isUpdating={isUpdating("fallback_enabled")}
        label={t("settings.kothalipi.engineControls.fallback.label")}
        description={t(
          "settings.kothalipi.engineControls.fallback.description",
        )}
        descriptionMode="tooltip"
        grouped={true}
      />

      {/* Active Engine & Live Priority Chain Card */}
      <div className="mx-4 my-2 p-3.5 rounded-lg border border-line bg-surface/40 flex flex-col gap-2.5">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-start gap-2.5">
            {!cloudIsPrimary ? (
              <div className="mt-0.5 p-1.5 rounded-md bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
                <Cpu size={16} />
              </div>
            ) : (
              <div className="mt-0.5 p-1.5 rounded-md bg-sky-500/10 text-sky-600 dark:text-sky-400">
                <Cloud size={16} />
              </div>
            )}
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold uppercase tracking-wider text-text/90">
                  {t("settings.kothalipi.status.activeEngine")}
                </span>
                {!cloudIsPrimary ? (
                  <span className="text-xs font-medium text-emerald-600 dark:text-emerald-400">
                    {t("settings.kothalipi.status.localOffline")}
                  </span>
                ) : activeCloudEngine ? (
                  <span className="text-xs font-medium text-sky-600 dark:text-sky-400">
                    {t("settings.kothalipi.status.cloudApi", {
                      provider: t(
                        `settings.kothalipi.cloud.providers.${activeCloudEngine.provider}`,
                      ),
                    })}
                  </span>
                ) : (
                  <span className="text-xs font-medium text-amber-600 dark:text-amber-400">
                    {t("settings.kothalipi.status.cloudFallback")}
                  </span>
                )}
              </div>
              <div className="text-xs text-text/70 mt-0.5">
                {!cloudIsPrimary ? (
                  <span>
                    {t("settings.kothalipi.status.voiceModel", {
                      name: currentModelInfo?.name || "Qwen3-ASR 0.6B Bangla",
                    })}
                  </span>
                ) : activeCloudEngine ? (
                  <span>
                    {t("settings.kothalipi.status.connectedApi", {
                      provider: t(
                        `settings.kothalipi.cloud.providers.${activeCloudEngine.provider}`,
                      ),
                      model: activeCloudEngine.model,
                    })}
                  </span>
                ) : (
                  <span>
                    {t("settings.kothalipi.status.noKeyFallback", {
                      name: currentModelInfo?.name || "Offline Model",
                    })}
                  </span>
                )}
              </div>
            </div>
          </div>

          <div className="shrink-0">
            {!cloudIsPrimary ? (
              <span className="text-[11px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-medium border border-emerald-500/20">
                {t("settings.kothalipi.status.badgeOnDevice")}
              </span>
            ) : activeCloudEngine ? (
              <span className="text-[11px] px-2 py-0.5 rounded-full bg-sky-500/10 text-sky-600 dark:text-sky-400 font-medium border border-sky-500/20">
                {t("settings.kothalipi.status.badgeApiActive")}
              </span>
            ) : (
              <span className="text-[11px] px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-600 dark:text-amber-400 font-medium border border-amber-500/20">
                {t("settings.kothalipi.status.badgeKeyNeeded")}
              </span>
            )}
          </div>
        </div>

        {/* Live Priority & Fallback Chain Visual (always visible!) */}
        <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-text/60 pt-2 border-t border-line/40">
          <span className="font-medium text-text/75">
            {t("settings.kothalipi.status.priority")}
          </span>
          {priorityChain.map((item, idx) => (
            <React.Fragment key={`${item.label}-${idx}`}>
              {idx > 0 && <span>→</span>}
              <span
                className={`px-1.5 py-0.5 rounded ${
                  item.active
                    ? "bg-text/10 text-text font-medium"
                    : "opacity-40 line-through"
                }`}
              >
                {idx + 1}. {item.label}
              </span>
            </React.Fragment>
          ))}
        </div>

        {/* AI Translation / Post-Processing indicator if enabled */}
        {postProcessEnabled && (
          <div className="flex items-center gap-1.5 text-xs text-purple-600 dark:text-purple-400 pt-2 border-t border-line/40">
            <Sparkles size={13} className="shrink-0" />
            <span>
              {t("settings.kothalipi.status.postProcessActive", {
                prompt: activePrompt?.name || "Active",
              })}
            </span>
          </div>
        )}
      </div>

      {/* Cloud engine rows are always visible so users can enable/disable or reorder Gemini/OpenAI in any mode */}
      {engines.map((engine, i) => (
        <EngineRow
          key={engine.provider}
          engine={engine}
          index={i}
          count={engines.length}
          hasKey={Boolean(keys[engine.provider])}
          onChange={(e) =>
            saveEngines(engines.map((x, j) => (j === i ? e : x)))
          }
          onMove={(delta) => {
            const next = [...engines];
            [next[i], next[i + delta]] = [next[i + delta], next[i]];
            saveEngines(next);
          }}
        />
      ))}

      {/* Claude explanation note */}
      <div className="mx-4 my-1.5 flex items-start gap-2 rounded-md bg-surface/30 px-3 py-2 text-xs text-text/70 border border-line/50">
        <Info size={14} className="mt-0.5 shrink-0 text-text/50" />
        <span>{t("onboarding.kothalipi.cloud.claudeNote")}</span>
      </div>

      <CloudUsagePanel providers={engines.map((e) => e.provider)} />
    </SettingsGroup>
  );
};
