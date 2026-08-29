import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useProjectStore, useUiStore } from "@/stores";
import { useTranslation } from "@/i18n";

export function CommandPalette() {
  const { t } = useTranslation();
  const open = useUiStore((s) => s.commandPaletteOpen);
  const setOpen = useUiStore((s) => s.setCommandPaletteOpen);
  const projects = useProjectStore((s) => s.projects);
  const [query, setQuery] = useState("");
  const navigate = useNavigate();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen(!open);
      }
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, setOpen]);

  if (!open) return null;

  const filtered = projects.filter((p) =>
    p.name.toLowerCase().includes(query.toLowerCase()),
  );

  return (
    <div className="command-palette-overlay" onClick={() => setOpen(false)}>
      <div className="command-palette" onClick={(e) => e.stopPropagation()}>
        <input
          autoFocus
          placeholder={t("commandPalette.placeholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <ul>
          {filtered.map((p) => (
            <li
              key={p.id}
              onClick={() => {
                navigate(`/projects/${p.id}`);
                setOpen(false);
              }}
            >
              {t("commandPalette.openProject", { name: p.name })}
            </li>
          ))}
          <li
            onClick={() => {
              navigate("/projects");
              setOpen(false);
            }}
          >
            {t("commandPalette.searchProjects")}
          </li>
          <li
            onClick={() => {
              navigate("/tasks");
              setOpen(false);
            }}
          >
            {t("commandPalette.createTask")}
          </li>
        </ul>
      </div>
    </div>
  );
}
