import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowDown, ArrowUp, Check } from "lucide-react";
import {
  commands,
  type CloudEngine,
  type CloudProvider,
  type ListenMode,
} from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { Dropdown } from "../ui/Dropdown";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";
import { SettingsGroup } from "../ui/SettingsGroup";

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
  const mode: ListenMode = getSetting("listen_mode") ?? "offline";
  const engines = getSetting("cloud_engines") ?? [];
  const keys = getSetting("cloud_api_keys") ?? {};

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
            { value: "offline", label: t("settings.kothalipi.mode.offline") },
            { value: "cloud", label: t("settings.kothalipi.mode.cloud") },
          ]}
          selectedValue={mode}
          onSelect={(v) => updateSetting("listen_mode", v as ListenMode)}
          disabled={isUpdating("listen_mode")}
        />
      </SettingContainer>
      {mode === "cloud" &&
        engines.map((engine, i) => (
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
    </SettingsGroup>
  );
};
