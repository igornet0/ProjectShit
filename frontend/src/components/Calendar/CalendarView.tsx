import { useMemo } from "react";
import type { CalendarEvent } from "@/types";
import { useTranslation, useLocaleStore } from "@/i18n";

interface CalendarProps {
  events: CalendarEvent[];
  month?: Date;
}

const WEEKDAY_KEYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const;

export function CalendarView({ events, month = new Date() }: CalendarProps) {
  const { t } = useTranslation();
  const locale = useLocaleStore((s) => s.locale);
  const days = useMemo(() => buildCalendarDays(month, events), [month, events]);

  const monthLabel = month.toLocaleDateString(locale, {
    month: "long",
    year: "numeric",
  });

  return (
    <div className="calendar panel">
      <h3 className="calendar-title">{monthLabel}</h3>
      <div className="calendar-grid">
        {WEEKDAY_KEYS.map((key) => (
          <div key={key} className="calendar-header">
            {t(`calendar.weekdays.${key}`)}
          </div>
        ))}
        {days.map((day) => (
          <div
            key={day.key}
            className={`calendar-day ${day.isCurrentMonth ? "" : "other-month"} ${day.isToday ? "today" : ""}`}
          >
            <span className="day-number">{day.date.getDate()}</span>
            {day.events.map((e) => (
              <div key={e.id} className="calendar-event">
                {e.title}
              </div>
            ))}
          </div>
        ))}
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
}

function buildCalendarDays(month: Date, events: CalendarEvent[]): CalendarDay[] {
  const year = month.getFullYear();
  const m = month.getMonth();
  const first = new Date(year, m, 1);
  const startDay = (first.getDay() + 6) % 7;
  const start = new Date(year, m, 1 - startDay);

  const today = new Date();
  const days: CalendarDay[] = [];

  for (let i = 0; i < 42; i++) {
    const date = new Date(start);
    date.setDate(start.getDate() + i);

    const dayEvents = events.filter((e) => {
      const d = new Date(e.start_at);
      return (
        d.getFullYear() === date.getFullYear() &&
        d.getMonth() === date.getMonth() &&
        d.getDate() === date.getDate()
      );
    });

    days.push({
      key: date.toISOString(),
      date,
      isCurrentMonth: date.getMonth() === m,
      isToday:
        date.getFullYear() === today.getFullYear() &&
        date.getMonth() === today.getMonth() &&
        date.getDate() === today.getDate(),
      events: dayEvents,
    });
  }

  return days;
}
