import { useCallback, useEffect, useState } from "react";
import { api } from "@/api";
import { Icon } from "@/components/Icon/Icon";
import { useLocaleStore, useTranslation } from "@/i18n";
import { formatRelative } from "@/utils/format";
import type { BrddBundle } from "@/types";

type Tab = "summary" | "changes" | "notes";

/** `.brdd` folder of a project: analysis, version snapshots, changelog, AI notes. */
export function BrddPanel({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const locale = useLocaleStore((s) => s.locale);
  const [bundle, setBundle] = useState<BrddBundle | null>(null);
  const [tab, setTab] = useState<Tab>("summary");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notes, setNotes] = useState("");
  const [notesDirty, setNotesDirty] = useState(false);

  const load = useCallback(async () => {
    try {
      const b = await api.brdd.get(projectId);
      setBundle(b);
      setNotes(b.notes_md ?? "");
      setNotesDirty(false);
    } catch (e) {
      setError(String(e));
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const refresh = async (force: boolean) => {
    setBusy(true);
    setError(null);
    try {
      await api.brdd.refresh(projectId, force);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const saveNotes = async () => {
    setBusy(true);
    setError(null);
    try {
      const b = await api.brdd.saveNotes(projectId, notes);
      setBundle(b);
      setNotesDirty(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const a = bundle?.analysis;
  const last = bundle?.versions[bundle.versions.length - 1];

  return (
    <section className="panel span-2 brdd-panel">
      <div className="panel-header">
        <h2>
          <Icon name="info" size={14} />
          .brdd
          {a?.version && <span className="badge accent">{a.version}</span>}
          {bundle && bundle.versions.length > 0 && (
            <span className="badge">{t("brdd.snapshots", { count: bundle.versions.length })}</span>
          )}
        </h2>
        <div className="button-row">
          <button type="button" className="btn" disabled={busy} onClick={() => void refresh(false)}>
            <Icon name="refresh" size={13} />
            {bundle?.exists ? t("brdd.refresh") : t("brdd.create")}
          </button>
          {bundle?.exists && (
            <button
              type="button"
              className="btn ghost"
              disabled={busy}
              title={t("brdd.snapshotHint")}
              onClick={() => void refresh(true)}
            >
              <Icon name="clock" size={13} />
              {t("brdd.snapshot")}
            </button>
          )}
        </div>
      </div>

      {error && <p className="brdd-error">{error}</p>}

      {!bundle?.exists ? (
        <p className="panel-empty">{t("brdd.empty")}</p>
      ) : (
        <>
          <div className="brdd-facts">
            {a?.stack.map((s) => (
              <span key={s} className="badge outline">
                {s}
              </span>
            ))}
            {a && (
              <span className="muted">
                {t("brdd.size", { files: a.structure.files, loc: a.structure.loc })}
              </span>
            )}
            {a && (
              <span className="muted">
                {t("brdd.tasks", { done: a.tasks.done, total: a.tasks.total })}
              </span>
            )}
            {a && <span className="muted">{t("brdd.analyzed", { when: formatRelative(a.analyzed_at, locale) })}</span>}
            {last && (
              <span className="muted">
                {t("brdd.lastSnapshot", { n: last.n, reason: t(`brdd.reason.${last.reason}`) })}
              </span>
            )}
          </div>

          <div className="brdd-tabs" role="tablist">
            {(["summary", "changes", "notes"] as const).map((id) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={tab === id}
                className={`brdd-tab${tab === id ? " active" : ""}`}
                onClick={() => setTab(id)}
              >
                {t(`brdd.tab.${id}`)}
              </button>
            ))}
          </div>

          {tab === "summary" && <pre className="brdd-md">{bundle.summary_md ?? "—"}</pre>}
          {tab === "changes" && <pre className="brdd-md">{bundle.changelog_md ?? "—"}</pre>}
          {tab === "notes" && (
            <div className="brdd-notes">
              <textarea
                value={notes}
                rows={8}
                placeholder={t("brdd.notesPlaceholder")}
                onChange={(e) => {
                  setNotes(e.target.value);
                  setNotesDirty(true);
                }}
              />
              <div className="button-row">
                <span className="muted">{t("brdd.notesHint")}</span>
                <button type="button" className="btn primary" disabled={busy || !notesDirty} onClick={() => void saveNotes()}>
                  {t("settings.save")}
                </button>
              </div>
            </div>
          )}
          <p className="brdd-path">
            <code title={bundle.dir}>{bundle.dir}</code>
          </p>
        </>
      )}
    </section>
  );
}
