import type { HubProjectEntry } from "@/types";
import { useTranslation } from "@/i18n";

interface GhostProjectCardProps {
  entry: HubProjectEntry;
  onClone: (entry: HubProjectEntry) => void;
  cloning?: boolean;
}

export function GhostProjectCard({
  entry,
  onClone,
  cloning,
}: GhostProjectCardProps) {
  const { t } = useTranslation();
  const { github_repo: repo } = entry;

  return (
    <article className="project-card ghost">
      <div className="project-card-link">
        <div className="project-card-icon">👻</div>
        <h3 className="project-card-name">{repo.full_name.toUpperCase()}</h3>
        <span className="ghost-badge">{t("projects.ghost")}</span>
        {repo.description && (
          <p className="project-card-language muted">{repo.description}</p>
        )}
        <p className="project-card-path muted">{repo.clone_url}</p>
      </div>
      <div className="project-card-actions">
        <button
          type="button"
          className="btn primary"
          disabled={cloning}
          onClick={() => onClone(entry)}
        >
          ⬇ {t("projects.install")}
        </button>
      </div>
    </article>
  );
}
