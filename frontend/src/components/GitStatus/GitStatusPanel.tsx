import type { GitCommitInfo, GitStatus } from "@/types";
import { useTranslation } from "@/i18n";

interface GitStatusProps {
  status: GitStatus | null;
  recentCommits: GitCommitInfo[];
  loading?: boolean;
}

export function GitStatusPanel({
  status,
  recentCommits,
  loading,
}: GitStatusProps) {
  const { t } = useTranslation();

  if (loading) {
    return <div className="git-panel loading">{t("git.loading")}</div>;
  }
  if (!status) {
    return <div className="git-panel empty">{t("git.notRepo")}</div>;
  }

  return (
    <div className="git-panel">
      <div className="git-branch">{status.branch}</div>
      <div className="git-stats">
        <span>● {t("git.modified", { count: status.modified })}</span>
        <span>↑ {t("git.ahead", { count: status.ahead })}</span>
        <span>↓ {t("git.behind", { count: status.behind })}</span>
        {status.staged > 0 && (
          <span>+ {t("git.staged", { count: status.staged })}</span>
        )}
        {status.untracked > 0 && (
          <span>? {t("git.untracked", { count: status.untracked })}</span>
        )}
      </div>
      {recentCommits.length > 0 && (
        <div className="git-commits">
          <h4>{t("git.recentCommits")}</h4>
          {recentCommits.slice(0, 5).map((c) => (
            <div key={c.hash} className="git-commit">
              <code>{c.hash}</code> {c.message}
              <span className="muted">{formatRelative(c.date, t)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function formatRelative(
  dateStr: string,
  t: (key: string, params?: Record<string, string | number>) => string,
): string {
  const date = new Date(dateStr);
  const diff = Date.now() - date.getTime();
  const hours = Math.floor(diff / 3600000);
  if (hours < 1) return t("git.justNow");
  if (hours < 24) return t("git.hoursAgo", { count: hours });
  const days = Math.floor(hours / 24);
  return t("git.daysAgo", { count: days });
}
