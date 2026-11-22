import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  CheckSquare,
  ChevronDown,
  ChevronUp,
  Copy,
  Mic,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { commands, type MeetingNote } from "@/bindings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Spinner } from "../../ui/Spinner";
import { notify } from "@/stores/noticeStore";

const formatDuration = (secs: number) => {
  const m = Math.floor(secs / 60)
    .toString()
    .padStart(2, "0");
  const s = (secs % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
};

const noteToMarkdown = (note: MeetingNote) => {
  const parts = [`# ${note.title}`, ""];
  if (note.summary) {
    parts.push("## Summary", note.summary, "");
  }
  if (note.key_points.length > 0) {
    parts.push("## Key Points", ...note.key_points.map((p) => `- ${p}`), "");
  }
  if (note.action_items.length > 0) {
    parts.push(
      "## Action Items",
      ...note.action_items.map((a) => `- [ ] ${a}`),
      "",
    );
  }
  parts.push("## Transcript", note.transcript);
  return parts.join("\n");
};

export const MeetingNotesSettings: React.FC = () => {
  const { t } = useTranslation();
  const [notes, setNotes] = useState<MeetingNote[]>([]);
  const [recording, setRecording] = useState(false);
  const [processing, setProcessing] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [title, setTitle] = useState("");
  const [expandedTranscript, setExpandedTranscript] = useState<
    Record<string, boolean>
  >({});
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const loadNotes = useCallback(async () => {
    const res = await commands.getMeetingNotes();
    if (res.status === "ok") setNotes(res.data);
  }, []);

  useEffect(() => {
    void loadNotes();
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [loadNotes]);

  const handleStart = async () => {
    const res = await commands.startMeetingRecording();
    if (res.status === "error") {
      notify({ title: res.error });
      return;
    }
    setRecording(true);
    setElapsed(0);
    timerRef.current = setInterval(() => {
      setElapsed((s) => s + 1);
    }, 1000);
  };

  const handleCancel = async () => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
    setRecording(false);
    setElapsed(0);
    await commands.cancelMeetingRecording();
  };

  const handleStopAndSummarize = async () => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
    setRecording(false);
    setProcessing(true);
    const res = await commands.stopAndSummarizeMeeting(
      title.trim() ? title.trim() : null,
      elapsed,
    );
    setProcessing(false);
    setElapsed(0);
    setTitle("");
    if (res.status === "error") {
      notify({ title: res.error });
      return;
    }
    setNotes((prev) => [res.data, ...prev]);
  };

  const handleDelete = async (id: string) => {
    const res = await commands.deleteMeetingNote(id);
    if (res.status === "ok") {
      setNotes((prev) => prev.filter((n) => n.id !== id));
    }
  };

  const handleCopy = async (note: MeetingNote) => {
    try {
      await navigator.clipboard.writeText(noteToMarkdown(note));
      setCopiedId(note.id);
      setTimeout(() => setCopiedId(null), 1800);
    } catch {
      notify({ title: t("settings.meetings.copyFailed") });
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="rounded-xl border border-line bg-surface/40 p-5 space-y-4">
        <div>
          <h2 className="text-base font-semibold text-text">
            {t("settings.meetings.title")}
          </h2>
          <p className="text-xs text-ink-2 mt-1">
            {t("settings.meetings.subtitle")}
          </p>
        </div>

        <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-2.5">
          <Input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t("settings.meetings.titlePlaceholder")}
            className="flex-1"
            disabled={processing}
          />
          {!recording && !processing && (
            <Button
              variant="primary"
              size="md"
              onClick={handleStart}
              className="inline-flex items-center gap-2 shrink-0"
            >
              <Mic size={15} aria-hidden="true" />
              {t("settings.meetings.startRecording")}
            </Button>
          )}
          {recording && (
            <div className="flex items-center gap-2 shrink-0">
              <span className="inline-flex items-center gap-1.5 rounded-full bg-red-500/15 px-3 py-1 text-xs font-semibold text-red-500 tabular-nums">
                <span className="size-2 rounded-full bg-red-500 animate-pulse" />
                {formatDuration(elapsed)}
              </span>
              <Button
                variant="primary"
                size="md"
                onClick={handleStopAndSummarize}
                className="inline-flex items-center gap-1.5"
              >
                <Square size={13} aria-hidden="true" />
                {t("settings.meetings.stopAndSummarize")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleCancel}
                aria-label={t("common.cancel")}
              >
                <X size={15} aria-hidden="true" />
              </Button>
            </div>
          )}
          {processing && (
            <span className="inline-flex items-center gap-2 text-xs font-medium text-ink-2 shrink-0">
              <Spinner />
              {t("settings.meetings.summarizing")}
            </span>
          )}
        </div>
      </div>

      {notes.length === 0 ? (
        <div className="rounded-xl border border-dashed border-line p-8 text-center text-sm text-ink-3">
          {t("settings.meetings.empty")}
        </div>
      ) : (
        <div className="space-y-4">
          {notes.map((note) => {
            const open = Boolean(expandedTranscript[note.id]);
            return (
              <div
                key={note.id}
                className="rounded-xl border border-line bg-surface/30 p-4 space-y-3"
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <h3 className="text-sm font-semibold text-text">
                      {note.title}
                    </h3>
                    <p className="text-[11px] text-ink-3 mt-0.5">
                      {new Date(note.created_at * 1000).toLocaleString()} ·{" "}
                      {formatDuration(note.duration_secs)}
                    </p>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleCopy(note)}
                      className="inline-flex items-center gap-1 text-xs"
                    >
                      {copiedId === note.id ? (
                        <Check size={13} className="text-emerald-500" />
                      ) : (
                        <Copy size={13} />
                      )}
                      {copiedId === note.id
                        ? t("settings.meetings.copied")
                        : t("settings.meetings.copy")}
                    </Button>
                    <button
                      type="button"
                      onClick={() => handleDelete(note.id)}
                      aria-label={t("settings.meetings.delete")}
                      className="p-1.5 rounded text-ink-3 hover:text-red-500 hover:bg-wash cursor-pointer"
                    >
                      <Trash2 size={14} aria-hidden="true" />
                    </button>
                  </div>
                </div>

                {note.summary && (
                  <p className="text-xs leading-relaxed text-text/90">
                    {note.summary}
                  </p>
                )}

                {note.key_points.length > 0 && (
                  <div className="space-y-1">
                    <div className="text-[11px] font-semibold uppercase tracking-wider text-ink-3">
                      {t("settings.meetings.keyPoints")}
                    </div>
                    <ul className="list-disc ps-5 text-xs text-text/85 space-y-0.5">
                      {note.key_points.map((pt, idx) => (
                        <li key={idx}>{pt}</li>
                      ))}
                    </ul>
                  </div>
                )}

                {note.action_items.length > 0 && (
                  <div className="space-y-1">
                    <div className="text-[11px] font-semibold uppercase tracking-wider text-ink-3">
                      {t("settings.meetings.actionItems")}
                    </div>
                    <ul className="space-y-1 text-xs text-text/90">
                      {note.action_items.map((item, idx) => (
                        <li key={idx} className="flex items-start gap-1.5">
                          <CheckSquare
                            size={13}
                            className="mt-0.5 shrink-0 text-logo-primary"
                            aria-hidden="true"
                          />
                          <span>{item}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                <div className="pt-2 border-t border-line/50">
                  <button
                    type="button"
                    onClick={() =>
                      setExpandedTranscript((prev) => ({
                        ...prev,
                        [note.id]: !open,
                      }))
                    }
                    className="inline-flex items-center gap-1 text-xs font-medium text-ink-2 hover:text-text cursor-pointer"
                  >
                    {open ? (
                      <ChevronUp size={14} aria-hidden="true" />
                    ) : (
                      <ChevronDown size={14} aria-hidden="true" />
                    )}
                    {t(
                      open
                        ? "settings.meetings.hideTranscript"
                        : "settings.meetings.showTranscript",
                    )}
                  </button>
                  {open && (
                    <p className="mt-2 whitespace-pre-wrap rounded-lg bg-wash/50 p-3 text-xs leading-relaxed text-text/85">
                      {note.transcript}
                    </p>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
