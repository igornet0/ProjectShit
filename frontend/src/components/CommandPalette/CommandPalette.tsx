import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Icon, type IconName } from "@/components/Icon/Icon";
import { useProjectStore, useUiStore } from "@/stores";
import { useTranslation } from "@/i18n";
import { languageColor, shortPath } from "@/utils/format";

interface PaletteItem {
  id: string;
  group: string;
  label: string;
  hint?: string;
  icon?: IconName;
  color?: string;
  run: () => void;
}

const MAX_PROJECTS = 30;

export function CommandPalette() {
  const { t } = useTranslation();
  const open = useUiStore((s) => s.commandPaletteOpen);
  const setOpen = useUiStore((s) => s.setCommandPaletteOpen);
  const projects = useProjectStore((s) => s.projects);
  const fetchProjects = useProjectStore((s) => s.fetchProjects);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);
  const navigate = useNavigate();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen(!open);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, setOpen]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      void fetchProjects();
    }
  }, [open, fetchProjects]);

  const items = useMemo<PaletteItem[]>(() => {
    const go = (path: string) => () => {
      navigate(path);
      setOpen(false);
    };
    const q = query.trim().toLowerCase();
    const matches = (text: string) => !q || text.toLowerCase().includes(q);

    const projectItems: PaletteItem[] = projects
      .filter((p) => matches(p.name) || matches(p.root_path))
      .slice(0, MAX_PROJECTS)
      .map((p) => ({
        id: `project-${p.id}`,
        group: t("nav.projects"),
        label: p.name,
        hint: shortPath(p.root_path, 30),
        color: languageColor(p.language),
        run: go(`/projects/${p.id}`),
      }));

    const actions: PaletteItem[] = [
      { id: "nav-dashboard", label: t("nav.dashboard"), icon: "dashboard" as const, run: go("/") },
      { id: "nav-projects", label: t("commandPalette.searchProjects"), icon: "folder" as const, run: go("/projects") },
      { id: "nav-tasks", label: t("commandPalette.createTask"), icon: "tasks" as const, run: go("/tasks") },
      { id: "nav-calendar", label: t("nav.calendar"), icon: "calendar" as const, run: go("/calendar") },
      { id: "nav-settings", label: t("nav.settings"), icon: "settings" as const, run: go("/settings") },
    ]
      .filter((a) => matches(a.label))
      .map((a) => ({ ...a, group: t("commandPalette.actions") }));

    return [...projectItems, ...actions];
  }, [projects, query, navigate, setOpen, t]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  useEffect(() => {
    listRef.current
      ?.querySelector<HTMLElement>(`[data-index="${activeIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  if (!open) return null;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      items[activeIndex]?.run();
    } else if (e.key === "Escape") {
      e.preventDefault();
      setOpen(false);
    }
  };

  return (
    <div className="palette-overlay" onMouseDown={() => setOpen(false)}>
      <div
        className="palette"
        role="dialog"
        aria-modal="true"
        aria-label={t("commandPalette.placeholder")}
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="palette-input">
          <Icon name="search" size={16} />
          <input
            autoFocus
            placeholder={t("commandPalette.placeholder")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            role="combobox"
            aria-expanded="true"
            aria-controls="palette-list"
            aria-activedescendant={items[activeIndex]?.id}
          />
          <kbd>esc</kbd>
        </div>

        {items.length === 0 ? (
          <p className="palette-empty">{t("commandPalette.empty")}</p>
        ) : (
          <ul className="palette-list" id="palette-list" role="listbox" ref={listRef}>
            {items.map((item, index) => (
              <PaletteRow
                key={item.id}
                item={item}
                index={index}
                active={index === activeIndex}
                showGroup={index === 0 || items[index - 1].group !== item.group}
                onHover={setActiveIndex}
              />
            ))}
          </ul>
        )}

        <div className="palette-footer">
          <span>
            <kbd>↑</kbd>
            <kbd>↓</kbd>
            {t("commandPalette.navigate")}
          </span>
          <span>
            <kbd>↵</kbd>
            {t("commandPalette.select")}
          </span>
          <span>
            <kbd>esc</kbd>
            {t("commandPalette.close")}
          </span>
        </div>
      </div>
    </div>
  );
}

function PaletteRow({
  item,
  index,
  active,
  showGroup,
  onHover,
}: {
  item: PaletteItem;
  index: number;
  active: boolean;
  showGroup: boolean;
  onHover: (index: number) => void;
}) {
  return (
    <>
      {showGroup && (
        <li className="palette-group" role="presentation">
          {item.group}
        </li>
      )}
      <li
        id={item.id}
        role="option"
        aria-selected={active}
        data-index={index}
        className={`palette-item${active ? " active" : ""}`}
        onMouseMove={() => !active && onHover(index)}
        onClick={item.run}
      >
        {item.icon ? (
          <Icon name={item.icon} size={15} />
        ) : (
          <span className="dot" style={{ "--dot-color": item.color } as React.CSSProperties} />
        )}
        <span className="palette-item-label">{item.label}</span>
        {item.hint && <span className="palette-item-hint">{item.hint}</span>}
      </li>
    </>
  );
}
