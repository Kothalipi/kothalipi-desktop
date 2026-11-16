import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertTriangle } from "lucide-react";
import { commands, type CloudProvider, type CloudUsage } from "@/bindings";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

const USAGE_URLS: Record<CloudProvider, string> = {
  gemini: "https://aistudio.google.com/usage",
  openai: "https://platform.openai.com/usage",
};

const fmtWait = (secs: number) =>
  secs < 90 ? `${Math.round(secs)} s` : `${Math.round(secs / 60)} min`;

/** What was sent to each cloud engine today, and any limit it said it hit. */
export const CloudUsagePanel: React.FC<{ providers: CloudProvider[] }> = ({
  providers,
}) => {
  const { t } = useTranslation();
  const [usage, setUsage] = useState<CloudUsage | null>(null);
  const [now, setNow] = useState(() => Date.now() / 1000);

  const load = useCallback(async () => {
    setUsage(await commands.getCloudUsage());
    setNow(Date.now() / 1000);
  }, []);

  useEffect(() => {
    load();
    const un = listen("cloud-usage-changed", load);
    const tick = setInterval(() => setNow(Date.now() / 1000), 5000);
    return () => {
      un.then((f) => f());
      clearInterval(tick);
    };
  }, [load]);

  const used = providers.filter((p) => usage?.providers[p]);

  return (
    <SettingContainer
      title={t("settings.kothalipi.cloud.usage.title")}
      description={t("settings.kothalipi.cloud.usage.note")}
      descriptionMode="tooltip"
      grouped={true}
      layout="stacked"
    >
      <div className="flex flex-col gap-2 text-sm">
        {used.length === 0 && (
          <span className="text-text/60">
            {t("settings.kothalipi.cloud.usage.none")}
          </span>
        )}
        {providers.map((p) => {
          const u = usage?.providers[p];
          const hit = usage?.limits[p];
          if (!u && !hit) return null;
          const name = t(`settings.kothalipi.cloud.providers.${p}`);
          const left =
            hit && hit.retry_after_secs != null
              ? hit.hit_at + hit.retry_after_secs - now
              : null;
          // A per-minute limit that has passed is no longer news.
          const stale = hit?.scope === "minute" && left != null && left < 0;
          return (
            <div key={p} className="flex flex-col gap-1">
              <div className="flex items-baseline justify-between gap-3">
                <span className="font-medium">{name}</span>
                {u && (
                  <span className="text-text/70">
                    {t("settings.kothalipi.cloud.usage.line", {
                      requests: u.requests,
                      minutes: (u.audio_secs / 60).toFixed(1),
                      failed: u.failed,
                    })}
                  </span>
                )}
              </div>
              {hit && !stale && (
                <div className="flex items-start gap-2 rounded-md bg-red-500/10 px-2 py-1.5 text-red-600">
                  <AlertTriangle size={14} className="mt-0.5 shrink-0" />
                  <span>
                    {hit.message}
                    {left != null && left > 0 && hit.scope === "minute"
                      ? ` ${t("settings.kothalipi.cloud.usage.retryIn", { time: fmtWait(left) })}`
                      : ""}
                  </span>
                </div>
              )}
            </div>
          );
        })}
        <div className="flex flex-wrap gap-2 pt-1">
          {providers.map((p) => (
            <Button
              key={p}
              variant="ghost"
              size="sm"
              onClick={() => openUrl(USAGE_URLS[p])}
            >
              {t(
                p === "gemini"
                  ? "settings.kothalipi.cloud.usage.openGemini"
                  : "settings.kothalipi.cloud.usage.openOpenai",
              )}
            </Button>
          ))}
        </div>
      </div>
    </SettingContainer>
  );
};
