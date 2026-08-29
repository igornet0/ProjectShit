import { useEffect } from "react";
import { CalendarView } from "@/components/Calendar/CalendarView";
import { useCalendarStore } from "@/stores";
import { useTranslation } from "@/i18n";

export function CalendarPage() {
  const { t } = useTranslation();
  const { events, fetchEvents, loading } = useCalendarStore();

  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  return (
    <div className="page calendar-page">
      <header className="page-header hero-header">
        <p className="eyebrow">{t("nav.calendar")}</p>
        <h1>{t("calendar.title")}</h1>
        <p className="muted">{t("calendar.subtitle")}</p>
      </header>
      {loading ? (
        <p className="muted">{t("common.loading")}</p>
      ) : (
        <CalendarView events={events} />
      )}
    </div>
  );
}
