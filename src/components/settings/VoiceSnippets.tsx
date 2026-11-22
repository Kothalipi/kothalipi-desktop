import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRight, Plus, Trash2 } from "lucide-react";
import { SettingsGroup } from "../ui/SettingsGroup";
import { SettingContainer } from "../ui/SettingContainer";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { CustomWords } from "./CustomWords";
import { useSettings } from "../../hooks/useSettings";
import type { VoiceSnippet } from "@/bindings";

/**
 * Unified Personal Dictionary (Custom Words) + Voice Snippets (Quick Triggers).
 */
export const VoiceSnippetsCard: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const snippets: VoiceSnippet[] = getSetting("voice_snippets") ?? [];

  const [trigger, setTrigger] = useState("");
  const [expansion, setExpansion] = useState("");

  const handleAdd = () => {
    const cleanTrigger = trigger.trim();
    const cleanExpansion = expansion.trim();
    if (!cleanTrigger || !cleanExpansion) return;
    const next: VoiceSnippet[] = [
      ...snippets,
      {
        id: `snip-${Date.now()}`,
        trigger: cleanTrigger,
        expansion: cleanExpansion,
      },
    ];
    updateSetting("voice_snippets", next);
    setTrigger("");
    setExpansion("");
  };

  const handleRemove = (id: string) => {
    updateSetting(
      "voice_snippets",
      snippets.filter((s) => s.id !== id),
    );
  };

  return (
    <SettingsGroup title={t("settings.kothalipi.dictionarySnippets.title")}>
      <CustomWords descriptionMode="tooltip" grouped={true} />

      <SettingContainer
        title={t("settings.kothalipi.dictionarySnippets.snippets.title")}
        description={t(
          "settings.kothalipi.dictionarySnippets.snippets.description",
        )}
        descriptionMode="tooltip"
        grouped={true}
        layout="stacked"
      >
        <div className="flex flex-col gap-2.5">
          <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-2">
            <Input
              type="text"
              value={trigger}
              onChange={(e) => setTrigger(e.target.value)}
              placeholder={t(
                "settings.kothalipi.dictionarySnippets.snippets.triggerPlaceholder",
              )}
              variant="compact"
              className="flex-1"
              disabled={isUpdating("voice_snippets")}
            />
            <Input
              type="text"
              value={expansion}
              onChange={(e) => setExpansion(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  handleAdd();
                }
              }}
              placeholder={t(
                "settings.kothalipi.dictionarySnippets.snippets.expansionPlaceholder",
              )}
              variant="compact"
              className="flex-1"
              disabled={isUpdating("voice_snippets")}
            />
            <Button
              variant="primary"
              size="md"
              onClick={handleAdd}
              disabled={
                !trigger.trim() ||
                !expansion.trim() ||
                isUpdating("voice_snippets")
              }
              className="inline-flex items-center gap-1 shrink-0"
            >
              <Plus size={14} aria-hidden="true" />
              {t("settings.kothalipi.dictionarySnippets.snippets.add")}
            </Button>
          </div>

          {snippets.length > 0 && (
            <div className="flex flex-col gap-1.5 mt-1">
              {snippets.map((s) => (
                <div
                  key={s.id}
                  className="flex items-center justify-between gap-2 rounded-lg border border-line bg-surface/40 px-3 py-2 text-xs"
                >
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    <span className="font-semibold text-text shrink-0">
                      “{s.trigger}”
                    </span>
                    <ArrowRight
                      size={12}
                      className="text-ink-3 shrink-0"
                      aria-hidden="true"
                    />
                    <span className="text-ink-2 truncate">{s.expansion}</span>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleRemove(s.id)}
                    aria-label={t(
                      "settings.kothalipi.dictionarySnippets.snippets.remove",
                    )}
                    className="p-1 rounded text-ink-3 hover:text-red-500 hover:bg-wash cursor-pointer shrink-0"
                  >
                    <Trash2 size={13} aria-hidden="true" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </SettingContainer>
    </SettingsGroup>
  );
};
