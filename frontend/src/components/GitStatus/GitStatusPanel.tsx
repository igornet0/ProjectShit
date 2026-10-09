import type { GitCommitInfo, GitStatus } from "@/types";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";
import { formatRelative } from "@/utils/format";

interface GitStatusProps {
  status: GitStatus | null;
  recentCommits: GitCommitInfo[];
  loading?: boolean;
}

export function GitStatusPanel({ status, recentCommits, loading }: GitStatusProps) {
  const { t, locale } = useTranslation();

  if (loading) {
    return <p className="panel-empty">{t("git.loading")}</p>;
  }
  if (!status) {
    return <p className="panel-empty">{t("git.notRepo")}</p>;
  }

  const clean = status.modified + status.staged + status.untracked === 0;

  return (
    <div className="git-panel">
      <div className="git-summary">
        <span className="git-branch">
          <Icon name="branch" size={14} />
          {status.branch}
        </span>
        {clean && (
          <span className="badge success">
            <Icon name="check" size={11} />
          </span>
        )}
        {status.modified > 0 && (
          <span className="badge warning">{t("git.modified", { count: status.modified })}</span>
        )}
        {status.staged > 0 && (
          <span className="badge success">{t("git.staged", { count: status.staged })}</span>
        )}
        {status.untracked > 0 && (
          <span className="badge">{t("git.untracked", { count: status.untracked })}</span>
        )}
        {status.ahead > 0 && (
          <span className="badge accent" title={t("git.ahead", { count: status.ahead })}>
            <Icon name="arrowUp" size={11} />
            {status.ahead}
          </span>
        )}
        {status.behind > 0 && (
          <span className="badge accent" title={t("git.behind", { count: status.behind })}>
            <Icon name="arrowDown" size={11} />
            {status.behind}
          </span>
        )}
      </div>

      {recentCommits.length > 0 && (
        <div className="git-commits" aria-label={t("git.recentCommits")}>
          {recentCommits.slice(0, 6).map((c) => (
            <div key={c.hash} className="git-commit" title={c.author}>
              <code>{c.hash}</code>
              <span className="git-commit-message">{c.message}</span>
              <span className="git-commit-time">{formatRelative(c.date, locale)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
