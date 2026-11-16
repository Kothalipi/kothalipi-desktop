/**
 * Kothalipi onboarding screens: choose offline or cloud, add cloud keys,
 * download the voice models, and set the language keys.
 */
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, Cloud, Cpu, Download, HardDrive, KeyRound } from "lucide-react";
import {
  commands,
  type CloudProvider,
  type DeviceProfile,
  type DeviceTier,
  type KeyConflicts,
  type ListenMode,
  type ModelInfo,
} from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { useModelStore } from "@/stores/modelStore";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Spinner } from "../../ui/Spinner";
import { ShortcutInput } from "../../settings/ShortcutInput";
import { OnboardingShell, ScreenHeader, ScreenMain } from "../OnboardingShell";

type Progress = { index: number; count: number };

export const BANGLA_MODEL_ID = "kothalipi-bn-qwen3-asr-0.6b";

/** The recommended English model: Parakeet, else Whisper Small. */
export const findEnglishModel = (models: ModelInfo[]) =>
  models.find(
    (m) =>
      m.id.includes("parakeet-unified-en-0.6b") && m.filename.includes("Q8_0"),
  ) ?? models.find((m) => m.id === "small");

const GEMINI_KEY_URL = "https://aistudio.google.com/apikey";
const OPENAI_KEY_URL = "https://platform.openai.com/api-keys";

const card =
  "flex w-full flex-col rounded-panel border border-line-strong px-[22px] text-start";

/* ---------------- Listen: offline or cloud ---------------- */

export const Listen: React.FC<{
  progress: Progress;
  onBack?: () => void;
  onContinue: (mode: ListenMode) => void;
}> = ({ progress, onBack, onContinue }) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const [mode, setMode] = useState<ListenMode>(
    getSetting("listen_mode") ?? "offline",
  );
  const device = useDeviceProfile();
  const suggest: ListenMode = device?.tier === "light" ? "cloud" : "offline";

  const option = (
    value: ListenMode,
    icon: React.ReactNode,
    tag: string,
    title: string,
    body: string,
  ) => (
    <button
      type="button"
      onClick={() => setMode(value)}
      aria-pressed={mode === value}
      className={`flex flex-1 flex-col gap-[6px] rounded-panel border p-[20px] text-start transition-colors ${
        mode === value
          ? "border-background-ui bg-wash"
          : "border-line-strong hover:border-ink-3"
      }`}
    >
      <span className="flex items-center gap-[8px] text-small font-semibold tracking-[0.08em] text-ink-3 uppercase">
        {icon}
        {tag}
      </span>
      <b className="font-display text-row font-[650]">{title}</b>
      <span className="text-small leading-[1.5] text-ink-2">{body}</span>
      {device && value === suggest && (
        <span className="mt-[4px] self-start rounded-full bg-moss-wash px-[8px] py-[1px] text-[11px] font-semibold text-moss">
          {t(
            value === "cloud"
              ? "onboarding.kothalipi.listen.suggestCloud"
              : "onboarding.kothalipi.listen.suggestOffline",
            { ram: device.ram_gb },
          )}
        </span>
      )}
    </button>
  );

  return (
    <OnboardingShell
      progress={progress}
      onBack={onBack}
      primary={
        <Button variant="primary" size="lg" onClick={() => onContinue(mode)}>
          {t("onboarding.shell.continue")}
        </Button>
      }
    >
      <ScreenMain
        header={
          <ScreenHeader
            title={t("onboarding.kothalipi.listen.title")}
            sub={t("onboarding.kothalipi.listen.sub")}
          />
        }
      >
        <div className="flex w-full gap-[14px]">
          {option(
            "offline",
            <HardDrive size={14} aria-hidden="true" />,
            t("onboarding.kothalipi.listen.offline.tag"),
            t("onboarding.kothalipi.listen.offline.title"),
            t("onboarding.kothalipi.listen.offline.body"),
          )}
          {option(
            "cloud",
            <Cloud size={14} aria-hidden="true" />,
            t("onboarding.kothalipi.listen.cloud.tag"),
            t("onboarding.kothalipi.listen.cloud.title"),
            t("onboarding.kothalipi.listen.cloud.body"),
          )}
        </div>
      </ScreenMain>
    </OnboardingShell>
  );
};

/* ---------------- Cloud keys ---------------- */

type KeyState = "idle" | "checking" | "ok" | "bad";

const KeyRow: React.FC<{
  provider: CloudProvider;
  name: string;
  note: string;
  getLabel: string;
  url: string;
  state: KeyState;
  error: string;
  onCheck: (key: string) => void;
}> = ({ provider, name, note, getLabel, url, state, error, onCheck }) => {
  const { t } = useTranslation();
  const [key, setKey] = useState("");
  return (
    <div className="flex flex-col gap-[8px] py-[16px] [&+&]:border-t [&+&]:border-line">
      <div className="flex items-baseline justify-between gap-[12px]">
        <div>
          <b className="block font-display text-row font-[650]">{name}</b>
          <span className="text-small text-ink-2">{note}</span>
        </div>
        <Button variant="ghost" size="sm" onClick={() => openUrl(url)}>
          {getLabel}
        </Button>
      </div>
      <div className="flex items-center gap-[8px]">
        <Input
          type="password"
          autoComplete="off"
          spellCheck={false}
          className="flex-1 select-text"
          aria-label={t("onboarding.kothalipi.cloud.keyLabel", { name })}
          placeholder={t("onboarding.kothalipi.cloud.placeholder")}
          value={key}
          onChange={(e) => setKey(e.target.value)}
          data-provider={provider}
        />
        {state === "ok" ? (
          <span className="inline-flex min-w-[96px] items-center justify-end gap-[6px] text-small font-semibold text-moss">
            <Check size={16} aria-hidden="true" />
            {t("onboarding.kothalipi.cloud.ok")}
          </span>
        ) : (
          <Button
            variant="secondary"
            size="md"
            className="min-w-[96px]"
            disabled={!key.trim() || state === "checking"}
            onClick={() => onCheck(key)}
          >
            {state === "checking" ? (
              <Spinner />
            ) : (
              t("onboarding.kothalipi.cloud.check")
            )}
          </Button>
        )}
      </div>
      {state === "bad" && (
        <span className="text-small text-red-500">{error}</span>
      )}
    </div>
  );
};

export const CloudKeys: React.FC<{
  progress: Progress;
  onBack: () => void;
  onContinue: () => void;
}> = ({ progress, onBack, onContinue }) => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const saved = getSetting("cloud_api_keys") ?? {};
  const [state, setState] = useState<Record<CloudProvider, KeyState>>({
    gemini: saved.gemini ? "ok" : "idle",
    openai: saved.openai ? "ok" : "idle",
  });
  const [errors, setErrors] = useState<Record<CloudProvider, string>>({
    gemini: "",
    openai: "",
  });

  const check = async (provider: CloudProvider, key: string) => {
    setState((s) => ({ ...s, [provider]: "checking" }));
    const result = await commands.checkCloudApiKey(provider, key);
    if (result.status === "ok") {
      await commands.changeCloudApiKeySetting(provider, key);
      await refreshSettings();
      setState((s) => ({ ...s, [provider]: "ok" }));
    } else {
      setErrors((e) => ({ ...e, [provider]: result.error }));
      setState((s) => ({ ...s, [provider]: "bad" }));
    }
  };

  const anyOk = state.gemini === "ok" || state.openai === "ok";

  return (
    <OnboardingShell
      progress={progress}
      onBack={onBack}
      hint={anyOk ? "" : t("onboarding.kothalipi.cloud.needOne")}
      primary={
        <Button
          variant="primary"
          size="lg"
          disabled={!anyOk}
          onClick={onContinue}
        >
          {t("onboarding.shell.continue")}
        </Button>
      }
    >
      <ScreenMain
        header={
          <ScreenHeader
            title={t("onboarding.kothalipi.cloud.title")}
            sub={t("onboarding.kothalipi.cloud.sub")}
          />
        }
      >
        <div className={card}>
          <KeyRow
            provider="gemini"
            name={t("onboarding.kothalipi.cloud.gemini.name")}
            note={t("onboarding.kothalipi.cloud.gemini.note")}
            getLabel={t("onboarding.kothalipi.cloud.getFreeKey")}
            url={GEMINI_KEY_URL}
            state={state.gemini}
            error={errors.gemini}
            onCheck={(k) => check("gemini", k)}
          />
          <KeyRow
            provider="openai"
            name={t("onboarding.kothalipi.cloud.openai.name")}
            note={t("onboarding.kothalipi.cloud.openai.note")}
            getLabel={t("onboarding.kothalipi.cloud.getKey")}
            url={OPENAI_KEY_URL}
            state={state.openai}
            error={errors.openai}
            onCheck={(k) => check("openai", k)}
          />
        </div>
        <p className="mt-[14px] mb-0 flex items-start gap-[8px] text-start text-small text-ink-3">
          <KeyRound
            size={14}
            className="mt-[3px] shrink-0"
            aria-hidden="true"
          />
          {t("onboarding.kothalipi.cloud.fallback")}{" "}
          {t("onboarding.kothalipi.cloud.claudeNote")}
        </p>
      </ScreenMain>
    </OnboardingShell>
  );
};

/* ---------------- Voice models ---------------- */

/** A model choice for one language: catalog id match and copy keys. */
interface Choice {
  match: (m: ModelInfo) => boolean;
  key: string;
}

const BANGLA_CHOICES: Choice[] = [
  { match: (m) => m.id === BANGLA_MODEL_ID, key: "qwen" },
  {
    match: (m) => m.id === "kothalipi-bn-bengaliai-whisper-medium",
    key: "bengaliai",
  },
];
const ENGLISH_CHOICES: Choice[] = [
  {
    match: (m) =>
      m.id.includes("parakeet-unified-en-0.6b") && m.filename.includes("Q8_0"),
    key: "parakeet",
  },
  {
    match: (m) =>
      m.id.includes("whisper-small.en") && m.filename.includes("Q8_0"),
    key: "whisperSmall",
  },
  {
    match: (m) =>
      m.id.includes("whisper-base.en") && m.filename.includes("Q8_0"),
    key: "whisperBase",
  },
];

/** Which choice fits this computer best. */
const recommendedKey = (lang: "bangla" | "english", tier: DeviceTier) =>
  lang === "bangla" ? "qwen" : tier === "light" ? "whisperSmall" : "parakeet";

/** This computer, read once per screen. */
export const useDeviceProfile = () => {
  const [profile, setProfile] = useState<DeviceProfile | null>(null);
  useEffect(() => {
    commands
      .getDeviceProfile()
      .then(setProfile)
      .catch(() => setProfile(null));
  }, []);
  return profile;
};

const LanguageModels: React.FC<{
  lang: "bangla" | "english";
  choices: Choice[];
  tier: DeviceTier;
  badge: string;
}> = ({ lang, choices, tier, badge }) => {
  const { t } = useTranslation();
  const models = useModelStore((s) => s.models);
  const progress = useModelStore((s) => s.downloadProgress);
  const downloading = useModelStore((s) => s.downloadingModels);
  const downloadModel = useModelStore((s) => s.downloadModel);
  const options = choices
    .map((c) => ({ ...c, model: models.find(c.match) }))
    .filter((o): o is Choice & { model: ModelInfo } => Boolean(o.model));
  const best = recommendedKey(lang, tier);
  const [picked, setPicked] = useState(best);
  const [expanded, setExpanded] = useState(false);
  const chosen = options.find((o) => o.key === picked) ?? options[0];
  // Only the pick shows until "Other models" opens the rest.
  const shown = expanded ? options : options.filter((o) => o === chosen);
  const base = `onboarding.kothalipi.voices.${lang}`;

  return (
    <div className="py-[14px] [&+&]:border-t [&+&]:border-line">
      <div className="mb-[8px] flex items-center gap-[8px]">
        <b className="font-display text-row font-[650]">{t(`${base}.name`)}</b>
        <span className="rounded-full bg-wash px-[8px] py-[1px] text-[11px] font-semibold text-ink-2">
          {badge}
        </span>
      </div>
      <div className="flex flex-col gap-[6px]" role="radiogroup">
        {shown.map((o) => {
          const ready = o.model.is_downloaded;
          const busy = Boolean(downloading[o.model.id]);
          const on = chosen?.key === o.key;
          return (
            <label
              key={o.key}
              className={`flex cursor-pointer items-center gap-[12px] rounded-[12px] border px-[12px] py-[10px] transition-colors ${
                on
                  ? "border-background-ui bg-wash"
                  : "border-line hover:border-ink-3"
              }`}
            >
              <input
                type="radio"
                name={lang}
                className="accent-[var(--color-background-ui)]"
                checked={on}
                onChange={() => setPicked(o.key)}
              />
              <span className="min-w-0 flex-1">
                <span className="flex flex-wrap items-center gap-[6px] text-small font-semibold">
                  {t(`${base}.options.${o.key}.name`)}
                  {o.key === best && (
                    <span className="rounded-full bg-moss-wash px-[7px] text-[11px] font-semibold text-moss">
                      {t("onboarding.kothalipi.voices.bestHere")}
                    </span>
                  )}
                </span>
                <span className="block text-[12.5px] leading-[1.4] text-ink-2">
                  {t(`${base}.options.${o.key}.note`)} ·{" "}
                  {t("onboarding.kothalipi.voices.size", {
                    mb: o.model.size_mb,
                  })}
                </span>
              </span>
              {ready ? (
                <span className="inline-flex items-center gap-[4px] text-small font-semibold text-moss">
                  <Check size={15} aria-hidden="true" />
                  {t("onboarding.kothalipi.voices.ready")}
                </span>
              ) : busy ? (
                <span className="inline-flex min-w-[64px] items-center justify-end gap-[6px] text-small text-ink-2 tabular-nums">
                  <Spinner />
                  {t("onboarding.kothalipi.voices.downloading", {
                    percent: Math.floor(progress[o.model.id]?.percentage ?? 0),
                  })}
                </span>
              ) : (
                on && (
                  <Button
                    variant="secondary"
                    size="sm"
                    className="inline-flex items-center gap-[6px]"
                    onClick={(e) => {
                      e.preventDefault();
                      downloadModel(o.model.id);
                    }}
                  >
                    <Download size={14} aria-hidden="true" />
                    {t("onboarding.kothalipi.voices.download")}
                  </Button>
                )
              )}
            </label>
          );
        })}
      </div>
      {options.length > 1 && (
        <button
          type="button"
          className="mt-[6px] cursor-pointer text-[12.5px] font-semibold text-ink-2 hover:text-text"
          onClick={() => setExpanded((x) => !x)}
          aria-expanded={expanded}
        >
          {t(
            expanded
              ? "onboarding.kothalipi.voices.fewer"
              : "onboarding.kothalipi.voices.more",
            { count: options.length - 1 },
          )}
        </button>
      )}
    </div>
  );
};

export const Voices: React.FC<{
  progress: Progress;
  cloud: boolean;
  onBack: () => void;
  onContinue: () => void;
}> = ({ progress, cloud, onBack, onContinue }) => {
  const { t } = useTranslation();
  const models = useModelStore((s) => s.models);
  const downloading = useModelStore((s) => s.downloadingModels);
  const currentModel = useModelStore((s) => s.currentModel);
  const selectModel = useModelStore((s) => s.selectModel);
  const device = useDeviceProfile();
  const tier: DeviceTier = device?.tier ?? "standard";

  const banglaModels = models.filter((m) =>
    BANGLA_CHOICES.some((c) => c.match(m)),
  );
  const banglaReady = banglaModels.find((m) => m.is_downloaded);
  const banglaStarted =
    Boolean(banglaReady) || banglaModels.some((m) => downloading[m.id]);

  // The plain transcribe key uses the selected model: pick Bangla once it's on disk.
  useEffect(() => {
    if (banglaReady && !currentModel) selectModel(banglaReady.id);
  }, [banglaReady?.id, currentModel, selectModel]);

  const canContinue = cloud || banglaStarted;
  const deviceLine = device
    ? t("onboarding.kothalipi.voices.device", {
        chip: device.chip || t("onboarding.kothalipi.voices.thisComputer"),
        ram: device.ram_gb,
      })
    : "";

  return (
    <OnboardingShell
      progress={progress}
      onBack={onBack}
      hint={
        canContinue
          ? t("onboarding.kothalipi.voices.continueWhile")
          : t("onboarding.kothalipi.voices.needBangla")
      }
      left={
        cloud && !banglaStarted ? (
          <Button variant="ghost" size="md" onClick={onContinue}>
            {t("onboarding.kothalipi.voices.skip")}
          </Button>
        ) : undefined
      }
      primary={
        <Button
          variant="primary"
          size="lg"
          disabled={!canContinue}
          onClick={onContinue}
        >
          {t("onboarding.shell.continue")}
        </Button>
      }
    >
      <ScreenMain
        header={
          <ScreenHeader
            title={t("onboarding.kothalipi.voices.title")}
            sub={
              <>
                {t(
                  cloud
                    ? "onboarding.kothalipi.voices.subCloud"
                    : "onboarding.kothalipi.voices.subOffline",
                )}
                {deviceLine && (
                  <span className="mt-[6px] flex items-center justify-center gap-[6px] text-small text-ink-3">
                    <Cpu size={13} aria-hidden="true" />
                    {deviceLine}
                    {tier === "light" &&
                      ` · ${t("onboarding.kothalipi.voices.lightHint")}`}
                  </span>
                )}
              </>
            }
          />
        }
      >
        <div className={card}>
          <LanguageModels
            lang="bangla"
            choices={BANGLA_CHOICES}
            tier={tier}
            badge={t(
              cloud
                ? "onboarding.kothalipi.voices.recommended"
                : "onboarding.kothalipi.voices.required",
            )}
          />
          <LanguageModels
            lang="english"
            choices={ENGLISH_CHOICES}
            tier={tier}
            badge={t("onboarding.kothalipi.voices.recommended")}
          />
        </div>
      </ScreenMain>
    </OnboardingShell>
  );
};

/* ---------------- Language keys ---------------- */

export const Keys: React.FC<{
  progress: Progress;
  isMac: boolean;
  onBack: () => void;
  onContinue: () => void;
}> = ({ progress, isMac, onBack, onContinue }) => {
  const { t } = useTranslation();
  const [conflicts, setConflicts] = useState<KeyConflicts | null>(null);

  const recheck = useCallback(async () => {
    if (!isMac) return;
    setConflicts(await commands.checkKeyConflicts());
  }, [isMac]);

  // Coming back from System Settings: check again.
  useEffect(() => {
    recheck();
    const off = getCurrentWindow().onFocusChanged(({ payload }) => {
      if (payload) recheck();
    });
    return () => {
      off.then((f) => f());
    };
  }, [recheck]);

  const problems = conflicts
    ? [
        conflicts.globe_key_busy && t("onboarding.kothalipi.keys.globe"),
        conflicts.dictation_on_right_command &&
          t("onboarding.kothalipi.keys.dictationRight"),
        conflicts.dictation_on_fn && t("onboarding.kothalipi.keys.dictationFn"),
      ].filter((p): p is string => Boolean(p))
    : [];

  return (
    <OnboardingShell
      progress={progress}
      onBack={onBack}
      hint={problems.length ? t("onboarding.kothalipi.keys.fixFirst") : ""}
      hintTone={problems.length ? "warning" : "quiet"}
      primary={
        <Button variant="primary" size="lg" onClick={onContinue}>
          {t("onboarding.shell.continue")}
        </Button>
      }
    >
      <ScreenMain
        header={
          <ScreenHeader
            title={t("onboarding.kothalipi.keys.title")}
            sub={t("onboarding.kothalipi.keys.sub")}
          />
        }
      >
        <div className="w-full text-start">
          <ShortcutInput shortcutId="transcribe_bangla" grouped={true} />
          <ShortcutInput shortcutId="transcribe_english" grouped={true} />
        </div>
        {isMac && problems.length > 0 && (
          <div className="mt-[14px] w-full rounded-panel border border-orange-400/60 bg-orange-400/10 p-[16px] text-start">
            <b className="text-small">
              {t("onboarding.kothalipi.keys.conflictsTitle")}
            </b>
            <ul className="mt-[6px] mb-[10px] ps-[18px] text-small text-ink-2">
              {problems.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
            <div className="flex gap-[8px]">
              <Button
                variant="primary"
                size="md"
                onClick={() => commands.openKeyboardSettings()}
              >
                {t("onboarding.kothalipi.keys.open")}
              </Button>
              <Button variant="ghost" size="md" onClick={recheck}>
                {t("onboarding.kothalipi.keys.recheck")}
              </Button>
            </div>
          </div>
        )}
        {isMac && conflicts && problems.length === 0 && (
          <p className="mt-[14px] mb-0 inline-flex items-center gap-[6px] text-small font-semibold text-moss">
            <Check size={16} aria-hidden="true" />
            {t("onboarding.kothalipi.keys.allClear")}
          </p>
        )}
      </ScreenMain>
    </OnboardingShell>
  );
};
