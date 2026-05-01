/**
 * DailyTrendChart Component
 *
 * Compact bar chart for 7-day activity.
 */

import React, { useMemo, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import type { DailyStatData } from "../types";
import { formatNumber } from "../utils";

interface DailyTrendChartProps {
  dailyData: DailyStatData[];
}

const BAR_HEIGHT = 48; // px

export const DailyTrendChart: React.FC<DailyTrendChartProps> = ({ dailyData }) => {
  const { t } = useTranslation();

  const today = useMemo(() => new Date().toISOString().split("T")[0], []);
  const [hoveredDate, setHoveredDate] = useState<string | null>(null);
  const [hoverRect, setHoverRect] = useState<DOMRect | null>(null);

  const handleBarHover = useCallback((date: string | null, rect: DOMRect | null) => {
    setHoveredDate(date);
    setHoverRect(rect);
  }, []);

  if (!dailyData.length) return null;

  const maxTokens = Math.max(...dailyData.map((d) => d.total_tokens), 1);
  const totalTokens = dailyData.reduce((sum, d) => sum + d.total_tokens, 0);
  const totalMessages = dailyData.reduce((sum, d) => sum + d.message_count, 0);
  const activeDays = dailyData.filter((d) => d.total_tokens > 0).length;

  const hoveredStat = hoveredDate ? dailyData.find((d) => d.date === hoveredDate) : null;

  const tooltipStyle = hoverRect && hoveredStat ? {
    position: "fixed" as const,
    left: hoverRect.left + hoverRect.width / 2,
    top: hoverRect.top - 8,
    transform: "translate(-50%, -100%)",
    zIndex: 50,
    pointerEvents: "none" as const,
  } : undefined;

  return (
    <div className="space-y-3">
      <div className="flex gap-2 overflow-x-auto pb-2 scrollbar-thin">
        {dailyData.map((stat) => {
          const isToday = stat.date === today;
          const ratio = stat.total_tokens / maxTokens;
          const barHeight = stat.total_tokens > 0 ? Math.max(ratio * BAR_HEIGHT, 4) : 2;
          const hasActivity = stat.total_tokens > 0;

          return (
            <div
              key={stat.date}
              className="flex-1 min-w-[12px] flex flex-col items-center cursor-pointer group"
              onMouseEnter={(e) => handleBarHover(stat.date, e.currentTarget.getBoundingClientRect())}
              onMouseLeave={() => handleBarHover(null, null)}
            >
              <div
                className="w-full flex items-end justify-center"
                style={{ height: `${BAR_HEIGHT}px` }}
              >
                <div
                  className="w-full max-w-[20px] rounded-t-sm transition-all duration-200 group-hover:brightness-110"
                  style={{
                    height: `${barHeight}px`,
                    backgroundColor: isToday
                      ? "#22c55e"
                      : hasActivity
                        ? "rgba(34, 197, 94, 0.5)"
                        : "rgba(128, 128, 128, 0.15)",
                  }}
                />
              </div>
              <span
                className="text-[9px] font-mono tabular-nums mt-1 whitespace-nowrap"
                style={{
                  fontWeight: isToday ? 600 : 400,
                  color: isToday ? "#22c55e" : "var(--muted-foreground)",
                  opacity: isToday ? 1 : 0.5,
                }}
              >
                {stat.date?.slice(8)}
              </span>
            </div>
          );
        })}
      </div>

      {hoveredStat && tooltipStyle && (
        <div
          style={tooltipStyle}
          className="bg-primary text-primary-foreground rounded-md px-3 py-1.5 text-xs shadow-lg"
        >
          <div className="font-medium">{hoveredStat.date}</div>
          <div>{t("analytics.tooltip.tokens")}: {formatNumber(hoveredStat.total_tokens)}</div>
          <div>{t("analytics.tooltip.messages")}: {hoveredStat.message_count}</div>
          <div>{t("analytics.tooltip.sessions")}: {hoveredStat.session_count}</div>
        </div>
      )}

      {/* Summary row */}
      <div className="flex items-center justify-between text-[10px] pt-2 border-t border-border/20">
        <div className="flex items-center gap-4">
          <div>
            <span className="text-muted-foreground">{t("analytics.dailyAvgTokens")}: </span>
            <span className="font-mono font-semibold text-foreground">{formatNumber(Math.round(totalTokens / dailyData.length))}</span>
          </div>
          <div>
            <span className="text-muted-foreground">{t("analytics.dailyAvgMessages")}: </span>
            <span className="font-mono font-semibold text-foreground">{Math.round(totalMessages / dailyData.length)}</span>
          </div>
        </div>
        <div className="flex items-center gap-1.5 text-muted-foreground/60">
          <div className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: "#22c55e" }} />
          <span>{activeDays}/{dailyData.length} {t("analytics.activeDays")}</span>
        </div>
      </div>
    </div>
  );
};

DailyTrendChart.displayName = "DailyTrendChart";
