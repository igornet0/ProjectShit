import { useMemo } from "react";
import type { CalendarEvent, Task } from "@/types";
import { useTranslation } from "@/i18n";
import { useUiStore } from "@/stores";

export interface ProjectedOccurrence {
  task: Task;
  at: Date;
}

interface CalendarProps {
  events: CalendarEvent[];
  tasks?: Task[];
  /** Future repeats of recurring tasks that do not exist as rows yet. */
  projected?: ProjectedOccurrence[];
  month?: Date;
}

const WEEKDAY_KEYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const;
const MAX_ITEMS_PER_DAY = 4;

export function CalendarView({
  events,
  tasks = [],
  projected = [],
  month = new Date(),
}: CalendarProps) {
  const { t, locale } = useTranslation();
  const openTask = useUiStore((s) => s.openTask);
  const days = useMemo(
    () => buildCalendarDays(month, events, tasks, projected),
    [month, events, tasks, projected],
  );

  const monthLabel = month.toLocaleDateString(locale, {
    month: "long",
    year: "numeric",
  });

  return (
    <div className="panel calendar">
      <div className="calendar-head">
        <h2>{monthLabel}</h2>
      </div>
      <div className="calendar-grid">
        {WEEKDAY_KEYS.map((key) => (
          <div key={key} className="calendar-weekday">
            {t(`calendar.weekdays.${key}`)}
          </div>
        ))}
        {days.map((day) => {
          const total = day.tasks.length + day.projected.length + day.events.length;
          const shownTasks = day.tasks.slice(0, MAX_ITEMS_PER_DAY);
          const shownProjected = day.projected.slice(0, MAX_ITEMS_PER_DAY - shownTasks.length);
          const shownEvents = day.events.slice(
            0,
            MAX_ITEMS_PER_DAY - shownTasks.length - shownProjected.length,
          );
          const hidden = total - shownTasks.length - shownProjected.length - shownEvents.length;
          return (
            <div
              key={day.key}
              className={`calendar-day${day.isCurrentMonth ? "" : " other-month"}${day.isToday ? " today" : ""}`}
            >
              <span className="day-number">{day.date.getDate()}</span>
              {shownTasks.map((task) => {
                const closed = task.status === "done" || task.status === "cancelled";
                const className = `calendar-task priority-${task.priority}${closed ? " closed" : ""}`;
                const title = `${task.title} · ${t(`enums.taskPriority.${task.priority}`)} · ${t(`enums.taskStatus.${task.status}`)}`;
                return (
                  <button
                    type="button"
                    key={task.id}
                    className={className}
                    title={title}
                    onClick={() => openTask(task.id)}
                  >
                    {task.title}
                  </button>
                );
              })}
              {shownProjected.map(({ task, at }) => (
                <button
                  type="button"
                  key={`${task.id}-${at.getTime()}`}
                  className={`calendar-task projected priority-${task.priority}`}
                  title={`${task.title} · ${t("tasks.repeatProjected")}`}
                  onClick={() => openTask(task.id)}
                >
                  {task.title}
                </button>
              ))}
              {shownEvents.map((e) => (
                <div key={e.id} className="calendar-event" title={e.title}>
                  {e.title}
                </div>
              ))}
              {hidden > 0 && <span className="calendar-more">+{hidden}</span>}
            </div>
          );
        })}
      </div>
    </div>
  );
}

interface CalendarDay {
  key: string;
  date: Date;
  isCurrentMonth: boolean;
  isToday: boolean;
  events: CalendarEvent[];
  tasks: Task[];
  projected: ProjectedOccurrence[];
}

const PRIORITY_ORDER: Record<Task["priority"], number> = {
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
};

function dayKey(d: Date): string {
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

function buildCalendarDays(
  month: Date,
  events: CalendarEvent[],
  tasks: Task[],
  projected: ProjectedOccurrence[],
): CalendarDay[] {
  const year = month.getFullYear();
  const m = month.getMonth();
  const first = new Date(year, m, 1);
  const startDay = (first.getDay() + 6) % 7;
  const todayKey = dayKey(new Date());

  // Bucket once by local day instead of filtering every list for each cell.
  const eventsByDay = new Map<string, CalendarEvent[]>();
  for (const e of events) {
    const key = dayKey(new Date(e.start_at));
    eventsByDay.set(key, [...(eventsByDay.get(key) ?? []), e]);
  }
  const tasksByDay = new Map<string, Task[]>();
  for (const task of tasks) {
    if (!task.due_at) continue;
    const key = dayKey(new Date(task.due_at));
    tasksByDay.set(key, [...(tasksByDay.get(key) ?? []), task]);
  }

  const projectedByDay = new Map<string, ProjectedOccurrence[]>();
  for (const item of projected) {
    const key = dayKey(item.at);
    projectedByDay.set(key, [...(projectedByDay.get(key) ?? []), item]);
  }

  const days: CalendarDay[] = [];
  for (let i = 0; i < 42; i++) {
    const date = new Date(year, m, 1 - startDay + i);
    const key = dayKey(date);
    days.push({
      key,
      date,
      isCurrentMonth: date.getMonth() === m,
      isToday: key === todayKey,
      events: eventsByDay.get(key) ?? [],
      projected: projectedByDay.get(key) ?? [],
      tasks: (tasksByDay.get(key) ?? []).sort(
        (a, b) => PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority],
      ),
    });
  }
  return days;
}
