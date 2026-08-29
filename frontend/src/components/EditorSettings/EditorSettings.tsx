import { useEffect, useState } from "react";
import { useEditorStore } from "@/stores";
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

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  const handleDefaultChange = async (editorId: string) => {
    if (!config) return;
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
    } finally {
      setSaving(false);
    }
  };

  if (loading && !config) {
    return <p className="muted">{t("common.loading")}</p>;
  }

  const editors = config?.editors ?? [];

  return (
    <div className="editor-settings">
      <label className="filter-field">
        <span>{t("settings.defaultEditor")}</span>
        <select
          value={config?.default_editor_id ?? ""}
          disabled={saving}
          onChange={(e) => handleDefaultChange(e.target.value)}
        >
          {editors.map((ed) => (
            <option key={ed.id} value={ed.id}>
              {ed.name}
              {!ed.builtin ? " (custom)" : ""}
            </option>
          ))}
        </select>
      </label>

      <ul className="simple-list editor-list">
        {editors.map((ed) => (
          <li key={ed.id}>
            <strong>{ed.name}</strong>
            <code className="editor-cmd">
              {ed.command} {ed.args.join(" ")}
            </code>
          </li>
        ))}
      </ul>

      <h3 className="editor-subheading">{t("settings.addCustomEditor")}</h3>
      <form className="editor-form" onSubmit={handleAddCustom}>
        <div className="editor-form-row">
          <label>
            <span>{t("settings.editorId")}</span>
            <input
              value={customId}
              onChange={(e) => setCustomId(e.target.value)}
              placeholder="webstorm"
              required
            />
          </label>
          <label>
            <span>{t("settings.editorName")}</span>
            <input
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
              placeholder="WebStorm"
              required
            />
          </label>
        </div>
        <div className="editor-form-row">
          <label>
            <span>{t("settings.editorCommand")}</span>
            <input
              value={customCommand}
              onChange={(e) => setCustomCommand(e.target.value)}
              placeholder="webstorm"
              required
            />
          </label>
          <label>
            <span>{t("settings.editorArgs")}</span>
            <input
              value={customArgs}
              onChange={(e) => setCustomArgs(e.target.value)}
              placeholder="{path}"
            />
          </label>
        </div>
        <button type="submit" className="btn primary" disabled={saving}>
          {t("settings.addCustomEditor")}
        </button>
      </form>
    </div>
  );
}
