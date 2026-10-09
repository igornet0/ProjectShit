import { useEffect, useState, useCallback } from "react";
import { useFolderStore, useProjectStore } from "@/stores";
import { useTranslation } from "@/i18n";
import { Icon } from "@/components/Icon/Icon";

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

  const renderView = (id: string, label: string, count: number) => (
    <li className="folder-row">
      <button
        type="button"
        className={`folder-item ${filterFolder === id ? "active" : ""}`}
        onClick={() => setFilterFolder(id)}
      >
        <Icon name="folder" size={14} />
        <span className="folder-item-name">{label}</span>
        <span className="folder-count">{count}</span>
      </button>
    </li>
  );

  return (
    <aside className="folder-sidebar" aria-label={t("folders.title")}>
      <div className="folder-sidebar-head">{t("folders.views")}</div>
      <ul className="folder-list">
        {renderView("all", t("folders.all"), totalProjects)}
        {renderView("none", t("folders.unassigned"), unassignedCount)}
      </ul>

      <div className="folder-sidebar-head">
        <span>{t("folders.myFolders")}</span>
        <button
          type="button"
          className="icon-btn small"
          onClick={() => {
            clearError();
            setCreating((v) => !v);
          }}
          title={t("folders.create")}
          aria-label={t("folders.create")}
          aria-expanded={creating}
        >
          <Icon name={creating ? "x" : "plus"} size={14} />
        </button>
      </div>

      {error && (
        <div className="folder-error" role="alert">
          {error}
        </div>
      )}

      {creating && (
        <form className="folder-create" onSubmit={handleCreate}>
          <input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setCreating(false);
                setNewName("");
              }
            }}
            placeholder={t("folders.namePlaceholder")}
            aria-label={t("folders.createTitle")}
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
        <p className="faint" style={{ padding: "4px 8px" }}>
          {t("common.loading")}
        </p>
      )}

      <ul className="folder-list">
        {folders.map((folder) => {
          const isDragging = dragId === folder.id;
          const isDropTarget = dropTargetId === folder.id;
          return (
            <li
              key={folder.id}
              className={`folder-row ${isDragging ? "dragging" : ""} ${isDropTarget ? "drop-target" : ""}`}
              onDragOver={handleDragOver(folder.id)}
              onDragLeave={handleDragLeave(folder.id)}
              onDrop={handleDrop(folder.id)}
            >
              <button
                type="button"
                className={`folder-item ${filterFolder === folder.id ? "active" : ""}`}
                draggable={false}
                onClick={() => setFilterFolder(folder.id)}
              >
                <Icon name="folder" size={14} />
                <span className="folder-item-name">{folder.name}</span>
                <span className="folder-count">{folder.project_count}</span>
              </button>
              <span className="folder-row-tools">
                <span
                  className="icon-btn small folder-drag-handle"
                  title={t("folders.dragHint")}
                  aria-label={t("folders.dragHint")}
                  draggable
                  onDragStart={handleDragStart(folder.id)}
                  onDragEnd={handleDragEnd}
                >
                  <Icon name="grip" size={13} />
                </span>
                <button
                  type="button"
                  className="icon-btn small danger"
                  title={t("common.remove")}
                  aria-label={t("common.remove")}
                  draggable={false}
                  onClick={() => handleDelete(folder.id)}
                >
                  <Icon name="x" size={13} />
                </button>
              </span>
            </li>
          );
        })}
      </ul>
    </aside>
  );
}
