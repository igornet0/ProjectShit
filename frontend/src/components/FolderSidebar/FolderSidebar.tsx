import { useEffect, useState, useCallback } from "react";
import { useFolderStore, useProjectStore } from "@/stores";
import { useTranslation } from "@/i18n";

export function FolderSidebar() {
  const { t } = useTranslation();
  const {
    folders,
    loading,
    error,
    fetchFolders,
    createFolder,
    deleteFolder,
    reorderFolders,
    clearError,
  } = useFolderStore();
  const { filterFolder, setFilterFolder, projects } = useProjectStore();
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [dragId, setDragId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

  useEffect(() => {
    fetchFolders();
  }, [fetchFolders]);

  const totalProjects = projects.length;
  const unassignedCount = projects.filter((p) => !p.folder_id).length;

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    const name = newName.trim();
    if (!name) return;

    setSubmitting(true);
    clearError();
    try {
      await createFolder(name);
      setNewName("");
      setCreating(false);
    } catch {
      // error is stored in folder store
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: string) => {
    clearError();
    try {
      await deleteFolder(id);
      if (filterFolder === id) {
        setFilterFolder("all");
      }
    } catch {
      // error is stored in folder store
    }
  };

  const moveFolder = useCallback(
    async (sourceId: string, targetId: string) => {
      if (sourceId === targetId) return;
      const ids = folders.map((f) => f.id);
      const from = ids.indexOf(sourceId);
      const to = ids.indexOf(targetId);
      if (from === -1 || to === -1) return;

      const next = [...ids];
      next.splice(from, 1);
      next.splice(to, 0, sourceId);
      await reorderFolders(next);
    },
    [folders, reorderFolders],
  );

  const handleDragStart = (id: string) => (e: React.DragEvent) => {
    e.stopPropagation();
    setDragId(id);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", id);
  };

  const handleDragOver = (id: string) => (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = "move";
    if (dragId && dragId !== id) {
      setDropTargetId(id);
    }
  };

  const handleDragLeave = (id: string) => (e: React.DragEvent) => {
    e.stopPropagation();
    const related = e.relatedTarget as Node | null;
    if (related && e.currentTarget.contains(related)) return;
    setDropTargetId((current) => (current === id ? null : current));
  };

  const handleDrop = (targetId: string) => async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const sourceId = dragId ?? e.dataTransfer.getData("text/plain");
    setDragId(null);
    setDropTargetId(null);
    if (!sourceId || sourceId === targetId) return;
    try {
      await moveFolder(sourceId, targetId);
    } catch {
      // error stored in folder store
    }
  };

  const handleDragEnd = () => {
    setDragId(null);
    setDropTargetId(null);
  };

  return (
    <aside className="folder-sidebar">
      <div className="folder-sidebar-top">
        <div>
          <p className="folder-sidebar-eyebrow">{t("nav.projects")}</p>
          <h2 className="folder-sidebar-title">{t("folders.title")}</h2>
        </div>
        <button
          type="button"
          className="folder-add-btn"
          onClick={() => {
            clearError();
            setCreating((v) => !v);
          }}
          aria-label={t("folders.create")}
          aria-expanded={creating}
        >
          {creating ? "×" : "+"}
        </button>
      </div>

      {error && (
        <div className="folder-error" role="alert">
          {error}
        </div>
      )}

      {creating && (
        <form className="folder-create-card" onSubmit={handleCreate}>
          <div className="folder-create-card-header">
            <span className="folder-create-icon">📁</span>
            <span>{t("folders.createTitle")}</span>
          </div>
          <input
            className="folder-create-input"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder={t("folders.namePlaceholder")}
            autoFocus
            disabled={submitting}
          />
          <div className="folder-create-actions">
            <button
              type="button"
              className="btn ghost small"
              disabled={submitting}
              onClick={() => {
                setCreating(false);
                setNewName("");
                clearError();
              }}
            >
              {t("common.cancel")}
            </button>
            <button
              type="submit"
              className="btn primary small"
              disabled={submitting || !newName.trim()}
            >
              {submitting ? t("common.loading") : t("common.add")}
            </button>
          </div>
        </form>
      )}

      {loading && folders.length === 0 && (
        <p className="muted folder-loading">{t("common.loading")}</p>
      )}

      <div className="folder-section">
        <p className="folder-section-label">{t("folders.views")}</p>
        <ul className="folder-list">
          <li>
            <button
              type="button"
              className={`folder-item ${filterFolder === "all" ? "active" : ""}`}
              onClick={() => setFilterFolder("all")}
            >
              <span className="folder-item-left">
                <span className="folder-item-icon">📂</span>
                <span className="folder-item-name">{t("folders.all")}</span>
              </span>
              <span className="folder-count">{totalProjects}</span>
            </button>
          </li>
          <li>
            <button
              type="button"
              className={`folder-item ${filterFolder === "none" ? "active" : ""}`}
              onClick={() => setFilterFolder("none")}
            >
              <span className="folder-item-left">
                <span className="folder-item-icon">📁</span>
                <span className="folder-item-name">{t("folders.unassigned")}</span>
              </span>
              <span className="folder-count">{unassignedCount}</span>
            </button>
          </li>
        </ul>
      </div>

      {folders.length > 0 && (
        <div className="folder-section">
          <p className="folder-section-label">{t("folders.myFolders")}</p>
          <p className="folder-drag-hint">{t("folders.dragHint")}</p>
          <ul className="folder-list folder-list--draggable">
            {folders.map((folder) => {
              const isDragging = dragId === folder.id;
              const isDropTarget = dropTargetId === folder.id;
              return (
                <li
                  key={folder.id}
                  className={`folder-list-row ${isDragging ? "dragging" : ""} ${isDropTarget ? "drop-target" : ""}`}
                  onDragOver={handleDragOver(folder.id)}
                  onDragLeave={handleDragLeave(folder.id)}
                  onDrop={handleDrop(folder.id)}
                >
                  <span
                    className="folder-drag-handle"
                    title={t("folders.dragHint")}
                    aria-label={t("folders.dragHint")}
                    draggable
                    onDragStart={handleDragStart(folder.id)}
                    onDragEnd={handleDragEnd}
                  >
                    ⠿
                  </span>
                  <button
                    type="button"
                    className={`folder-item ${filterFolder === folder.id ? "active" : ""}`}
                    draggable={false}
                    onClick={() => setFilterFolder(folder.id)}
                  >
                    <span className="folder-item-left">
                      <span className="folder-item-icon">🗂️</span>
                      <span className="folder-item-name">{folder.name}</span>
                    </span>
                    <span className="folder-count">{folder.project_count}</span>
                  </button>
                  <button
                    type="button"
                    className="folder-delete-btn"
                    title={t("common.remove")}
                    draggable={false}
                    onClick={() => handleDelete(folder.id)}
                  >
                    ✕
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </aside>
  );
}
