import { useEffect, useState } from "react";
import { useEditorStore } from "@/stores";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";

export function EditorSettings() {
  const { t } = useTranslation();
  const { config, loading, fetchConfig, saveConfig, addCustomEditor } =
    useEditorStore();

  const [customId, setCustomId] = useState("");
  const [customName, setCustomName] = useState("");
  const [customCommand, setCustomCommand] = useState("");
  const [customArgs, setCustomArgs] = useState("{path}");
  const [saving, setSaving] = useState(false);
  const [showAddForm, setShowAddForm] = useState(false);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  const handleDefaultChange = async (editorId: string) => {
    if (!config || config.default_editor_id === editorId) return;
    setSaving(true);
    try {
      await saveConfig({ ...config, default_editor_id: editorId });
    } finally {
      setSaving(false);
    }
  };

  const handleAddCustom = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!customId.trim() || !customName.trim() || !customCommand.trim()) return;

    setSaving(true);
    try {
      await addCustomEditor({
        id: customId.trim(),
        name: customName.trim(),
        command: customCommand.trim(),
        args: customArgs
          .split(",")
          .map((a) => a.trim())
          .filter(Boolean),
      });
      setCustomId("");
      setCustomName("");
      setCustomCommand("");
      setCustomArgs("{path}");
      setShowAddForm(false);
    } finally {
      setSaving(false);
    }
  };

  if (loading && !config) {
    return <p className="muted">{t("common.loading")}</p>;
  }

  const editors = config?.editors ?? [];
  const defaultId = config?.default_editor_id ?? "";

  return (
    <div className="editor-settings">
      <p className="settings-section-label">{t("settings.installedEditors")}</p>
      <ul className="choice-list" role="radiogroup" aria-label={t("settings.defaultEditor")}>
        {editors.map((ed) => {
          const isDefault = ed.id === defaultId;
          return (
            <li key={ed.id}>
              <button
                type="button"
                role="radio"
                aria-checked={isDefault}
                className={`choice${isDefault ? " active" : ""}`}
                disabled={saving}
                onClick={() => handleDefaultChange(ed.id)}
              >
                <span className="choice-radio" aria-hidden="true" />
                <span className="choice-main">
                  <span className="choice-title">
                    {ed.name}
                    {isDefault && <span className="badge accent">{t("settings.defaultBadge")}</span>}
                    {!ed.builtin && <span className="badge outline">custom</span>}
                  </span>
                  <code className="choice-sub mono">
                    {ed.command} {ed.args.join(" ")}
                  </code>
                </span>
              </button>
            </li>
          );
        })}
      </ul>

      {!showAddForm ? (
        <button
          type="button"
          className="btn add-row-btn"
          onClick={() => setShowAddForm(true)}
        >
          <Icon name="plus" size={14} />
          {t("settings.addCustomEditor")}
        </button>
      ) : (
        <form className="editor-add-form" onSubmit={handleAddCustom}>
          <div className="editor-add-form-head">
            <h3>{t("settings.addCustomEditor")}</h3>
          </div>
          <div className="form-grid">
            <label className="field">
              <span>{t("settings.editorId")}</span>
              <input
                value={customId}
                onChange={(e) => setCustomId(e.target.value)}
                placeholder="webstorm"
                required
              />
            </label>
            <label className="field">
              <span>{t("settings.editorName")}</span>
              <input
                value={customName}
                onChange={(e) => setCustomName(e.target.value)}
                placeholder="WebStorm"
                required
              />
            </label>
            <label className="field">
              <span>{t("settings.editorCommand")}</span>
              <input
                className="mono"
                value={customCommand}
                onChange={(e) => setCustomCommand(e.target.value)}
                placeholder="webstorm"
                required
              />
            </label>
            <label className="field">
              <span>{t("settings.editorArgs")}</span>
              <input
                className="mono"
                value={customArgs}
                onChange={(e) => setCustomArgs(e.target.value)}
                placeholder="{path}"
              />
            </label>
          </div>
          <div className="form-actions">
            <button type="button" className="btn ghost" onClick={() => setShowAddForm(false)}>
              {t("common.cancel")}
            </button>
            <button type="submit" className="btn primary" disabled={saving}>
              {t("common.add")}
            </button>
          </div>
        </form>
      )}
    </div>
  );
}
