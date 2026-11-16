import React, { useState, useEffect } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";

import ModelSelector from "../model-selector";
import UpdateChecker from "../update-checker";
import NoticeChip from "./NoticeChip";
import TranscriptionStatus from "./TranscriptionStatus";

interface FooterProps {
  onShowHistory: () => void;
}

const Footer: React.FC<FooterProps> = ({ onShowHistory }) => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.2");
      }
    };

    fetchVersion();
  }, []);

  return (
    <div className="w-full border-t border-line pt-3">
      <div className="flex justify-between items-center text-xs px-4 pb-3 text-ink-2">
        <div className="flex items-center gap-4">
          <ModelSelector />
          <TranscriptionStatus onShowHistory={onShowHistory} />
        </div>

        <div className="flex items-center gap-3">
          {/* Update Status */}
          <div className="flex items-center gap-1">
            <UpdateChecker />
            <span>•</span>
            {/* eslint-disable-next-line i18next/no-literal-string */}
            <span>v{version}</span>
            <span>•</span>
            <button
              type="button"
              className="cursor-pointer hover:text-text transition-colors"
              onClick={() => openUrl("https://tashanto.com")}
            >
              {t("footer.madeBy")}
            </button>
          </div>
          <NoticeChip />
        </div>
      </div>
    </div>
  );
};

export default Footer;
