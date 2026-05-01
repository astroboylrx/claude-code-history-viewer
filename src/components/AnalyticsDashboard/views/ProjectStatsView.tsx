/**
 * ProjectStatsView Component
 *
 * Displays project-level analytics and statistics.
 */

import React, { useMemo, useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { MessageCircle, Activity, Clock, Wrench, Layers, Cpu, TrendingUp, Database } from "lucide-react";
import { LoadingState } from "@/components/ui/loading";
import type { ProjectStatsSummary, ProviderId } from "../../../types";
import { formatDuration } from "../../../utils/time";
import {
  MetricCard,
  SectionCard,
  BillingBreakdownCard,
  ActivityHeatmapComponent,
  ToolUsageChart,
  DailyTrendChart,
  TokenDistributionChart,
} from "../components";
import { formatNumber, formatCurrency, generateTrendData, extractProjectGrowth } from "../utils";
import { calculateGlobalCostSummary, calculateModelMetrics } from "../utils/globalCalculations";
import { supportsConversationBreakdown } from "../../../utils/providers";

interface ProjectStatsViewProps {
  projectSummary: ProjectStatsSummary | null;
  conversationSummary: ProjectStatsSummary | null;
  providerId?: ProviderId;
}

export const ProjectStatsView: React.FC<ProjectStatsViewProps> = ({
  projectSummary,
  conversationSummary,
  providerId = "claude",
}) => {
  const { t } = useTranslation();

  const [showCharts, setShowCharts] = useState(false);

  useEffect(() => {
    if (!projectSummary) {
      setShowCharts(false);
      return;
    }
    const id = requestAnimationFrame(() => setShowCharts(true));
    return () => cancelAnimationFrame(id);
  }, [projectSummary]);

  // Generate full range daily data using utility function
  const dailyData = useMemo(
    () => generateTrendData(projectSummary?.daily_stats),
    [projectSummary?.daily_stats]
  );

  // Calculate billing cost from model distribution
  const billingCostSummary = useMemo(() => {
    if (!projectSummary?.model_distribution?.length) {
      return { totalEstimatedCost: 0 };
    }
    return calculateGlobalCostSummary(
      projectSummary.model_distribution,
      projectSummary.total_tokens
    );
  }, [projectSummary?.model_distribution, projectSummary?.total_tokens]);

  // Calculate conversation cost if conversation summary is available
  const conversationCost = useMemo(() => {
    if (!conversationSummary?.model_distribution?.length) return null;
    const summary = calculateGlobalCostSummary(
      conversationSummary.model_distribution,
      conversationSummary.total_tokens
    );
    return summary.totalEstimatedCost;
  }, [conversationSummary?.model_distribution, conversationSummary?.total_tokens]);

  // 데이터가 없으면 항상 로딩 상태 표시 (뷰 전환 직후 isLoading이 false일 수 있음)
  if (!projectSummary) {
    return (
      <div className="flex items-center justify-center h-full min-h-[400px]">
        <LoadingState
          isLoading={true}
          loadingMessage={t("analytics.loading")}
          spinnerSize="lg"
          withSparkle={true}
        />
      </div>
    );
  }

  // Calculate growth metrics using utility function
  const { tokenGrowth, messageGrowth } = extractProjectGrowth(projectSummary);
  const billingTokens = projectSummary.total_tokens;
  const billingCost = billingCostSummary.totalEstimatedCost;

  return (
    <div className="space-y-6">
      {/* Metric Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          icon={MessageCircle}
          label={t("analytics.totalMessages")}
          value={formatNumber(projectSummary.total_messages)}
          trend={messageGrowth}
          colorVariant="purple"
        />
        <MetricCard
          icon={Activity}
          label={t("analytics.totalTokens")}
          value={formatNumber(projectSummary.total_tokens)}
          trend={tokenGrowth}
          subValue={
            billingCost > 0
              ? t("analytics.estimatedCostValue", "Estimated Cost: {{cost}}", {
                  cost: formatCurrency(billingCost),
                })
              : t("analytics.sessionCount", "{{count}} sessions", {
                  count: projectSummary.total_sessions,
                })
          }
          colorVariant="blue"
        />
        <MetricCard
          icon={Clock}
          label={t("analytics.totalSessionTime")}
          value={formatDuration(projectSummary.total_session_duration)}
          subValue={`${t("analytics.avgSessionTime", "Avg Session Time")}: ${formatDuration(
            projectSummary.avg_session_duration
          )}`}
          colorVariant="green"
        />
        <MetricCard
          icon={Wrench}
          label={t("analytics.toolsUsed")}
          value={projectSummary.most_used_tools.length}
          colorVariant="amber"
        />
      </div>

      <BillingBreakdownCard
        billingTokens={billingTokens}
        conversationTokens={conversationSummary != null ? conversationSummary.total_tokens : null}
        billingCost={billingCost}
        conversationCost={conversationCost}
        showProviderLimitHelp={!supportsConversationBreakdown(providerId)}
      />

      {showCharts && (<>
      {/* Charts Row 1 */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
        <SectionCard title={t("analytics.activityHeatmapTitle")} icon={Layers} colorVariant="green">
          {projectSummary.daily_stats.length > 0 ? (
            <ActivityHeatmapComponent data={projectSummary.daily_stats} />
          ) : (
            <div className="text-center py-8 text-muted-foreground text-[12px]">
              {t("analytics.No activity data available")}
            </div>
          )}
        </SectionCard>

        <SectionCard title={t("analytics.mostUsedToolsTitle")} icon={Cpu} colorVariant="purple">
          <ToolUsageChart tools={projectSummary.most_used_tools} />
        </SectionCard>
      </div>

      {/* Daily Trend Chart */}
      {projectSummary.daily_stats.length > 0 && (
        <SectionCard title={t("analytics.recentActivityTrend")} icon={TrendingUp} colorVariant="blue">
          <DailyTrendChart dailyData={dailyData} />
        </SectionCard>
      )}

      {/* Token Distribution + Model Distribution */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
        <SectionCard title={t("analytics.tokenTypeDistribution")} icon={Database} colorVariant="amber">
          <TokenDistributionChart
            distribution={projectSummary.token_distribution}
            total={projectSummary.total_tokens}
          />
        </SectionCard>

        {projectSummary.model_distribution.length > 0 && (
          <SectionCard title={t("analytics.modelDistribution")} icon={Cpu} colorVariant="blue">
            <div className="space-y-3">
              {projectSummary.model_distribution.map((model) => {
                const { percentage, formattedPrice, formattedTokens } = calculateModelMetrics(
                  model.model_name,
                  model.token_count,
                  model.input_tokens,
                  model.output_tokens,
                  model.cache_creation_tokens,
                  model.cache_read_tokens,
                  projectSummary.total_tokens
                );

                return (
                  <div key={model.model_name}>
                    <div className="flex items-center justify-between mb-1.5">
                      <span className="block max-w-[60%] text-[12px] font-medium text-foreground truncate">
                        {model.model_name}
                      </span>
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[12px] text-muted-foreground">
                          {formattedPrice}
                        </span>
                        <span className="font-mono text-[12px] font-semibold text-foreground">
                          {formattedTokens}
                        </span>
                      </div>
                    </div>
                    <div className="h-2 bg-muted/30 rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full"
                        style={{
                          width: `${percentage}%`,
                          background:
                            "linear-gradient(90deg, var(--metric-purple), var(--metric-blue))",
                        }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </SectionCard>
        )}
      </div>
      </>)}
    </div>
  );
};

ProjectStatsView.displayName = "ProjectStatsView";
