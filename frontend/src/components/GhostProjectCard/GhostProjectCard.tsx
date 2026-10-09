import type { HubProjectEntry } from "@/types";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";

interface GhostProjectCardProps {
  entry: HubProjectEntry;
  onClone: (entry: HubProjectEntry) => void;
  cloning?: boolean;
}

/** A GitHub repository that is not cloned locally, rendered as a table row. */
export function GhostProjectCard({ entry, onClone, cloning }: GhostProjectCardProps) {
  const { t } = useTranslation();
  const { github_repo: repo } = entry;

  return (
    <div className="project-row ghost" role="row">
      <div className="project-name-cell" role="cell">
        <Icon name="github" size={15} />
        <span className="project-name" title={repo.clone_url}>
          {repo.full_name}
        </span>
        <span className="badge outline">{t("projects.ghost")}</span>
      </div>
      <span className="project-path" role="cell" title={repo.description ?? repo.clone_url}>
        {repo.description ?? repo.clone_url}
      </span>
      <div className="project-actions" role="cell">
        <button
          type="button"
          className="btn small"
          disabled={cloning}
          onClick={() => onClone(entry)}
        >
          <Icon name={cloning ? "refresh" : "download"} size={13} />
          {t("projects.install")}
        </button>
      </div>
    </div>
  );
}
