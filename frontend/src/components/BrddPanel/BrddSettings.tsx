import { useEffect, useState } from "react";
import { api } from "@/api";
import { useTranslation } from "@/i18n";
import type { BrddRefreshAll, BrddSettings as Settings, IssueSyncReport } from "@/types";

/** Settings → .brdd: automation flags, bulk refresh, GitHub issue sync. */
export function BrddSettings() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    api.brdd.getSettings().then(setSettings).catch((e) => setMessage(String(e)));
  }, []);

  const save = async (next: Settings) => {
    setSettings(next);
    try {
      setSettings(await api.brdd.saveSettings(next));
    } catch (e) {
      setMessage(String(e));
    }
  };

  const run = async (key: string, fn: () => Promise<string>) => {
    setBusy(key);
    setMessage(null);
    try {
      setMessage(await fn());
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(null);
    }
  };

  if (!settings) return <p className="muted">{message ?? "…"}</p>;

  return (
    <div className="brdd-settings">
      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={settings.auto_on_scan}
          onChange={(e) => void save({ ...settings, auto_on_scan: e.target.checked })}
        />
        <span>
          <strong>{t("brdd.autoOnScan")}</strong>
          <br />
          <span className="muted">{t("brdd.autoOnScanHint")}</span>
        </span>
      </label>
      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={settings.exclude_from_git}
          onChange={(e) => void save({ ...settings, exclude_from_git: e.target.checked })}
        />
        <span>
          <strong>{t("brdd.excludeFromGit")}</strong>
          <br />
          <span className="muted">{t("brdd.excludeFromGitHint")}</span>
        </span>
      </label>
      <div className="button-row">
        <button
          type="button"
          className="btn"
          disabled={busy !== null}
          onClick={() =>
            void run("all", async () => {
              const r: BrddRefreshAll = await api.brdd.refreshAll();
              return t("brdd.refreshAllDone", { ok: r.refreshed, failed: r.failed });
            })
          }
        >
          {busy === "all" ? "…" : t("brdd.refreshAll")}
        </button>
        <button
          type="button"
          className="btn"
          disabled={busy !== null}
          onClick={() =>
            void run("issues", async () => {
              const r: IssueSyncReport = await api.github.syncIssues();
              return t("brdd.issuesSynced", {
                checked: r.checked,
                done: r.tasks_completed,
                closed: r.issues_closed,
              });
            })
          }
        >
          {busy === "issues" ? "…" : t("brdd.syncIssues")}
        </button>
      </div>
      {message && <p className="muted">{message}</p>}
    </div>
  );
}
