import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface ShowDockIconProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/** macOS only: Kothalipi lives in the menu bar; this adds it to the Dock too. */
export const ShowDockIcon: React.FC<ShowDockIconProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    if (type() !== "macos") return null;

    return (
      <ToggleSwitch
        checked={getSetting("show_dock_icon") ?? false}
        onChange={(enabled) => updateSetting("show_dock_icon", enabled)}
        isUpdating={isUpdating("show_dock_icon")}
        label={t("settings.advanced.showDockIcon.label")}
        description={t("settings.advanced.showDockIcon.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        tooltipPosition="bottom"
      />
    );
  },
);
