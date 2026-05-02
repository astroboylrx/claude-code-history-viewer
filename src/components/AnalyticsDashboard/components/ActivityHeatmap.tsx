/**
 * ActivityHeatmap Component
 *
 * Monthly calendar-style heatmap showing daily activity.
 * Each month is rendered as a separate block with day-of-week headers.
 */

import React, { useMemo, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { DailyStats } from "../../../types";
import { formatNumber, getHeatColor } from "../utils";

interface ActivityHeatmapProps {
  data: DailyStats[];
}

/** Parse "YYYY-MM-DD" into a local Date (returns epoch fallback on invalid input) */
function parseDate(dateStr: string): Date {
  const [y, m, d] = dateStr.split("-").map(Number);
  const year = y ?? 0;
  const month = m ?? 1;
  const day = d ?? 1;
  if (!Number.isFinite(year) || !Number.isFinite(month) || !Number.isFinite(day)) {
    return new Date(0);
  }
  return new Date(year, month - 1, day);
}

/** Group DailyStats by "YYYY-MM" key, sorted chronologically */
function groupByMonth(data: DailyStats[]): Map<string, DailyStats[]> {
  const map = new Map<string, DailyStats[]>();
  for (const entry of data) {
    const key = entry.date.slice(0, 7); // "YYYY-MM"
    const arr = map.get(key) ?? [];
    arr.push(entry);
    map.set(key, arr);
  }
  // Sort keys chronologically
  const sorted = new Map(
    [...map.entries()].sort(([a], [b]) => a.localeCompare(b))
  );
  return sorted;
}

interface CalendarDay {
  date: string | null; // null for padding cells
  dayNum: number; // day-of-month (1-31), 0 for padding cells
  messageCount: number;
  sessionCount: number;
  totalTokens: number;
}

const EMPTY_CELL: CalendarDay = { date: null, dayNum: 0, messageCount: 0, sessionCount: 0, totalTokens: 0 };

/** Build a calendar grid for a given month */
function buildMonthGrid(
  yearMonth: string,
  entries: DailyStats[]
): CalendarDay[][] {
  const [year, month] = yearMonth.split("-").map(Number);
  const y = year ?? 2000;
  const m = (month ?? 1) - 1;

  const firstDay = new Date(y, m, 1);
  const lastDay = new Date(y, m + 1, 0);
  const daysInMonth = lastDay.getDate();
  const startDow = firstDay.getDay(); // 0=Sun

  // Build lookup
  const lookup = new Map<number, DailyStats>();
  for (const entry of entries) {
    const dayNum = parseDate(entry.date).getDate();
    lookup.set(dayNum, entry);
  }

  const weeks: CalendarDay[][] = [];
  let currentWeek: CalendarDay[] = [];

  // Pad leading empty cells
  for (let i = 0; i < startDow; i++) {
    currentWeek.push(EMPTY_CELL);
  }

  for (let day = 1; day <= daysInMonth; day++) {
    const stats = lookup.get(day);
    const dateStr = `${y}-${String(m + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
    currentWeek.push({
      date: dateStr,
      dayNum: day,
      messageCount: stats?.message_count ?? 0,
      sessionCount: stats?.session_count ?? 0,
      totalTokens: stats?.total_tokens ?? 0,
    });

    if (currentWeek.length === 7) {
      weeks.push(currentWeek);
      currentWeek = [];
    }
  }

  // Pad trailing empty cells
  if (currentWeek.length > 0) {
    while (currentWeek.length < 7) {
      currentWeek.push(EMPTY_CELL);
    }
    weeks.push(currentWeek);
  }

  return weeks;
}

/** Format "YYYY-MM" into a localized month+year label */
function formatMonthLabel(yearMonth: string): string {
  const [year, month] = yearMonth.split("-").map(Number);
  const date = new Date(year ?? 2000, (month ?? 1) - 1, 1);
  return new Intl.DateTimeFormat(undefined, { year: "numeric", month: "short" }).format(date);
}

const MonthBlock: React.FC<{
  yearMonth: string;
  weeks: CalendarDay[][];
  maxActivity: number;
  weekdayLabels: string[];
  onHover: (cell: CalendarDay | null, rect: DOMRect | null) => void;
}> = React.memo(({ yearMonth, weeks, maxActivity, weekdayLabels, onHover }) => {
  const monthLabel = formatMonthLabel(yearMonth);

  return (
    <div className="flex flex-col gap-1">
      <div className="text-xs font-semibold text-foreground/80 mb-0.5">
        {monthLabel}
      </div>

      <div className="grid grid-cols-7 gap-px">
        {weekdayLabels.map((label, i) => (
          <div
            key={i}
            className="w-[14px] h-[14px] flex items-center justify-center text-4xs font-medium text-muted-foreground/50"
          >
            {label}
          </div>
        ))}
      </div>

      {weeks.map((week, weekIdx) => (
        <div key={weekIdx} className="grid grid-cols-7 gap-px">
          {week.map((cell, dayIdx) => {
            if (cell.date == null) {
              return <div key={dayIdx} className="w-[14px] h-[14px]" />;
            }

            const intensity = maxActivity > 0 ? cell.messageCount / maxActivity : 0;
            const heatColor = getHeatColor(intensity);

            return (
              <div
                key={dayIdx}
                className={cn(
                  "w-[14px] h-[14px] rounded-sm",
                  intensity > 0 && "cursor-pointer hover:scale-125 hover:z-10 hover:ring-1 hover:ring-white/30"
                )}
                style={{ backgroundColor: heatColor }}
                onMouseEnter={(e) => onHover(cell, e.currentTarget.getBoundingClientRect())}
                onMouseLeave={() => onHover(null, null)}
              >
                {cell.dayNum === 1 && (
                  <span className="text-[6px] text-foreground/40 leading-none flex items-center justify-center h-full">
                    1
                  </span>
                )}
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
});

MonthBlock.displayName = "MonthBlock";

export const ActivityHeatmapComponent: React.FC<ActivityHeatmapProps> = React.memo(({ data }) => {
  const { t } = useTranslation();
  const weekdayLabels = t("analytics.weekdayNamesShort", { returnObjects: true }) as string[];
  const [hoveredCell, setHoveredCell] = useState<CalendarDay | null>(null);
  const [tooltipRect, setTooltipRect] = useState<DOMRect | null>(null);

  const handleHover = useCallback((cell: CalendarDay | null, rect: DOMRect | null) => {
    setHoveredCell(cell);
    setTooltipRect(rect);
  }, []);

  const { months, maxActivity, totalMessages } = useMemo(() => {
    const grouped = groupByMonth(data);
    let max = 0;
    let total = 0;

    const monthEntries: Array<{ key: string; weeks: CalendarDay[][] }> = [];
    for (const [key, entries] of grouped) {
      const weeks = buildMonthGrid(key, entries);
      for (const week of weeks) {
        for (const cell of week) {
          if (cell.messageCount > max) max = cell.messageCount;
          total += cell.messageCount;
        }
      }
      monthEntries.push({ key, weeks });
    }

    return { months: monthEntries, maxActivity: Math.max(max, 1), totalMessages: total };
  }, [data]);

  const tooltipStyle = tooltipRect && hoveredCell?.date ? {
    position: "fixed" as const,
    left: tooltipRect.left + tooltipRect.width / 2,
    top: tooltipRect.top - 8,
    transform: "translate(-50%, -100%)",
    zIndex: 50,
    pointerEvents: "none" as const,
  } : undefined;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap gap-4">
        {months.map(({ key, weeks }) => (
          <MonthBlock
            key={key}
            yearMonth={key}
            weeks={weeks}
            maxActivity={maxActivity}
            weekdayLabels={weekdayLabels}
            onHover={handleHover}
          />
        ))}
      </div>

      {hoveredCell?.date && tooltipStyle && (
        <div
          style={tooltipStyle}
          className="bg-primary text-primary-foreground rounded-md px-3 py-1.5 text-xs shadow-lg"
        >
          <div className="font-medium mb-1">{hoveredCell.date}</div>
          <div>{t("analytics.tooltip.messages")}: {hoveredCell.messageCount}</div>
          <div>{t("analytics.tooltip.sessions")}: {hoveredCell.sessionCount}</div>
          <div>{t("analytics.tooltip.tokens")}: {formatNumber(hoveredCell.totalTokens)}</div>
        </div>
      )}

      <div className="flex items-center justify-between pt-3 border-t border-border/30">
        <div className="flex items-center gap-2">
          <span className="text-3xs font-medium text-muted-foreground">
            {t("analytics.legend.less")}
          </span>
          <div className="flex gap-0.5">
            {[0, 0.25, 0.5, 0.75, 1].map((intensity) => (
              <div
                key={intensity}
                className="w-3 h-3 rounded-sm"
                style={{ backgroundColor: getHeatColor(intensity) }}
              />
            ))}
          </div>
          <span className="text-3xs font-medium text-muted-foreground">
            {t("analytics.legend.more")}
          </span>
        </div>

        <span className="text-3xs font-mono text-muted-foreground">
          {t("analytics.calendarTotal", { count: totalMessages })}
        </span>
      </div>
    </div>
  );
});

ActivityHeatmapComponent.displayName = "ActivityHeatmapComponent";
