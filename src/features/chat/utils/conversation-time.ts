export function formatConversationListTime(value?: string, locale?: string): string {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const startOfWeek = new Date(startOfToday);
  const dayOfWeek = startOfToday.getDay();
  const diffToMonday = dayOfWeek === 0 ? 6 : dayOfWeek - 1;
  startOfWeek.setDate(startOfWeek.getDate() - diffToMonday);

  if (date >= startOfToday) {
    return date.toLocaleTimeString(locale, {
      hour: "2-digit",
      minute: "2-digit",
    });
  }
  if (date >= startOfWeek) {
    return date.toLocaleDateString(locale, {
      weekday: "short",
    });
  }
  if (date.getFullYear() === now.getFullYear()) {
    return date.toLocaleDateString(locale, {
      month: "2-digit",
      day: "2-digit",
    });
  }
  return date.toLocaleDateString(locale, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

export function formatConversationListTimeWithMinuteDetails(
  value?: string,
  locale?: string,
): { dateLabel: string; timeLabel: string } {
  if (!value) {
    return { dateLabel: "", timeLabel: "" };
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return { dateLabel: value, timeLabel: "" };
  }
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const nextStartOfToday = new Date(startOfToday);
  nextStartOfToday.setDate(nextStartOfToday.getDate() + 1);
  const isToday = date >= startOfToday && date < nextStartOfToday;
  return {
    dateLabel: formatConversationListTime(value, locale),
    timeLabel: isToday ? "" : date.toLocaleTimeString(locale, {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }),
  };
}
