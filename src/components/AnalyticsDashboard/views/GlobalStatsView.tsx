/**
 * GlobalStatsView Component
 *
 * Displays global statistics across all projects.
 */

import React, { useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
  Activity,
  MessageCircle,
  Clock,
  Wrench,
  Cpu,
  Layers,
  BarChart3,
  Server,
} from "lucide-react";
import type { GlobalStatsSummary } from "../../../types";
import { formatDuration } from "../../../utils/time";
import { cn } from "@/lib/utils";
import {
  MetricCard,
  SectionCard,
  BillingBreakdownCard,
  ActivityHeatmapComponent,
  ToolUsageChart,
  ProviderDistributionChart,
} from "../components";
import {
  formatNumber,
  formatCurrency,
  calculateModelMetrics,
  calculateGlobalCostSummary,
  getRankMedal,
  hasMedal,
} from "../utils";
import { calculateConversationBreakdownCoverage } from "../../../utils/providers";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../ui/tooltip";
import { useAppStore } from "../../../store/useAppStore";

interface GlobalStatsViewProps {
  globalSummary: GlobalStatsSummary;
  globalConversationSummary: GlobalStatsSummary | null;
}

export const GlobalStatsView: React.FC<GlobalStatsViewProps> = ({
  globalSummary,
  globalConversationSummary,
}) => {
  const { t } = useTranslation();
  const projects = useAppStore((s) => s.projects);

  const handleProjectClick = useCallback((projectName: string, providerId: string) => {
    const project = projects.find((p) => {
      const nameMatch = p.name === projectName || p.actual_path === projectName || p.path === projectName;
      if (!nameMatch) return false;
      return (p.provider ?? "claude") === providerId;
    });
    if (!project) return;
    useAppStore.getState().requestProjectNavigation(project);
  }, [projects]);
  const totalSessionTime = globalSummary.total_session_duration_minutes;
  const costSummary = useMemo(
    () =>
      calculateGlobalCostSummary(
        globalSummary.model_distribution,
        globalSummary.total_tokens
      ),
    [globalSummary.model_distribution, globalSummary.total_tokens]
  );
  const totalEstimatedCost = costSummary.totalEstimatedCost;
  const conversationCostSummary = useMemo(() => {
    if (!globalConversationSummary) {
      return null;
    }
    return calculateGlobalCostSummary(
      globalConversationSummary.model_distribution,
      globalConversationSummary.total_tokens
    );
  }, [globalConversationSummary]);

  const billingTokens = globalSummary.total_tokens;
  const billingCost = totalEstimatedCost;
  const conversationBreakdownCoverage = useMemo(
    () =>
      calculateConversationBreakdownCoverage(globalSummary.provider_distribution),
    [globalSummary.provider_distribution]
  );

  const lastUpdated = useMemo(() => {
    const raw = globalSummary.date_range.last_message;
    if (!raw) {
      return t("analytics.lastUpdatedUnknown", "Unknown");
    }
    const parsed = new Date(raw);
    if (Number.isNaN(parsed.getTime())) {
      return t("analytics.lastUpdatedUnknown", "Unknown");
    }
    return new Intl.DateTimeFormat(undefined, {
      year: "numeric",
      month: "short",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(parsed);
  }, [globalSummary.date_range.last_message, t]);

  return (
    <div className="flex-1 p-3 md:p-6 overflow-auto bg-background space-y-4 md:space-y-6">
      <p className="text-xs text-muted-foreground">
        {t(
          "analytics.providerScopeProjectTree",
          "Provider scope follows Project Tree provider tabs."
        )}
      </p>

      {/* Metric Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 md:gap-4">
        <MetricCard
          icon={Activity}
          label={t("analytics.totalTokens")}
          value={formatNumber(globalSummary.total_tokens)}
          subValue={
            t("analytics.estimatedCostValue", "Estimated Cost: {{cost}}", {
              cost: formatCurrency(totalEstimatedCost),
            })
          }
          colorVariant="blue"
        />
        <MetricCard
          icon={MessageCircle}
          label={t("analytics.totalMessages")}
          value={formatNumber(globalSummary.total_messages)}
          subValue={`${t("analytics.totalSessions")}: ${globalSummary.total_sessions}`}
          colorVariant="purple"
        />
        <MetricCard
          icon={Clock}
          label={t("analytics.sessionTime")}
          value={formatDuration(totalSessionTime)}
          colorVariant="green"
        />
        <MetricCard
          icon={Wrench}
          label={t("analytics.toolsUsed")}
          value={globalSummary.most_used_tools.length}
          colorVariant="amber"
        />
      </div>

      <BillingBreakdownCard
        billingTokens={billingTokens}
        conversationTokens={globalConversationSummary?.total_tokens ?? null}
        billingCost={billingCost}
        conversationCost={conversationCostSummary?.totalEstimatedCost ?? null}
        showProviderLimitHelp={conversationBreakdownCoverage.hasLimitedProviders}
      />

      <div className="flex flex-wrap items-center gap-2">
        <span className="px-2 py-1 rounded-md bg-amber-500/10 text-amber-700 dark:text-amber-300 text-xs">
          {t("analytics.estimatedLabel", "Estimated")}
        </span>
        <span className="px-2 py-1 rounded-md bg-muted/40 text-muted-foreground text-xs">
          {t("analytics.pricingCoverage", "Pricing coverage")}: {costSummary.coveragePercent.toFixed(1)}%
        </span>
        <span className="px-2 py-1 rounded-md bg-muted/40 text-muted-foreground text-xs">
          {t("analytics.lastUpdated", "Last updated")}: {lastUpdated}
        </span>
      </div>

      {/* Model Distribution & Tool Usage */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
        {globalSummary.provider_distribution.length > 0 && (
          <SectionCard
            title={t("analytics.providerDistribution", "Provider Distribution")}
            icon={Server}
            colorVariant="green"
          >
            <ProviderDistributionChart providers={globalSummary.provider_distribution} />
          </SectionCard>
        )}

        {globalSummary.model_distribution.length > 0 && (
          <SectionCard title={t("analytics.modelDistribution")} icon={Cpu} colorVariant="blue">
            <div className="space-y-3">
              {globalSummary.model_distribution.map((model) => {
                const { percentage, formattedPrice, formattedTokens } = calculateModelMetrics(
                  model.model_name,
                  model.token_count,
                  model.input_tokens,
                  model.output_tokens,
                  model.cache_creation_tokens,
                  model.cache_read_tokens,
                  globalSummary.total_tokens
                );

                return (
                  <div key={model.model_name}>
                    <div className="flex items-center justify-between mb-1.5">
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <button
                            type="button"
                            className="block max-w-[60%] text-sm font-medium text-foreground truncate text-left cursor-default"
                          >
                            {model.model_name}
                          </button>
                        </TooltipTrigger>
                        <TooltipContent>
                          {model.model_name}
                        </TooltipContent>
                      </Tooltip>
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-sm text-muted-foreground">
                          {formattedPrice}
                        </span>
                        <span className="font-mono text-sm font-semibold text-foreground">
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

      {/* Heatmap & Top Projects */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
        <SectionCard title={t("analytics.activityHeatmapTitle")} icon={Layers} colorVariant="green">
          {globalSummary.daily_stats.length > 0 ? (
            <ActivityHeatmapComponent data={globalSummary.daily_stats} />
          ) : (
            <div className="text-center py-8 text-muted-foreground text-sm">
              {t("analytics.No activity data available")}
            </div>
          )}
        </SectionCard>

        {globalSummary.top_projects.length > 0 && (
          <SectionCard title={t("analytics.topProjects")} icon={BarChart3} colorVariant="purple">
            <div className="space-y-2">
              {globalSummary.top_projects.slice(0, 8).map((project, index) => {
                const medal = getRankMedal(index);
                return (
                  <div
                    key={`${project.project_name}-${project.provider_id}`}
                    className={cn(
                      "flex items-center justify-between p-2.5 rounded-lg cursor-pointer",
                      "bg-muted/30 hover:bg-muted/50 transition-colors"
                    )}
                    onClick={() => handleProjectClick(project.project_name, project.provider_id)}
                  >
                    <div className="flex items-center gap-3 flex-1 min-w-0">
                      <div
                        className={cn(
                          "w-6 h-6 rounded-md flex items-center justify-center text-sm font-bold",
                          hasMedal(index) ? "text-base" : "bg-muted text-muted-foreground"
                        )}
                      >
                        {medal ?? index + 1}
                      </div>
                      <div className="flex-1 min-w-0">
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <span
                              className="block w-full text-sm font-medium text-foreground truncate"
                            >
                              {project.project_name}
                            </span>
                          </TooltipTrigger>
                          <TooltipContent>
                            {project.project_name}
                          </TooltipContent>
                        </Tooltip>
                        <p className="text-sm text-muted-foreground flex items-center gap-1.5 flex-wrap">
                          <span
                            className={cn(
                              "inline-flex px-1.5 py-0.5 text-2xs font-medium rounded-full leading-none",
                              project.provider_id === "claude" && "bg-amber-500/15 text-amber-700 dark:text-amber-300",
                              project.provider_id === "codex" && "bg-green-500/15 text-green-600 dark:text-green-400",
                              project.provider_id === "kimi" && "bg-orange-500/15 text-orange-600 dark:text-orange-400",
                              project.provider_id === "opencode" && "bg-blue-500/15 text-blue-600 dark:text-blue-400",
                              project.provider_id === "gemini" && "bg-purple-500/15 text-purple-600 dark:text-purple-400",
                              project.provider_id === "forgecode" && "bg-violet-500/15 text-violet-600 dark:text-violet-400",
                              project.provider_id === "antigravity" && "bg-pink-500/15 text-pink-600 dark:text-pink-400",
                              project.provider_id === "cline" && "bg-teal-500/15 text-teal-600 dark:text-teal-400",
                            )}
                          >
                            {project.provider_id}
                          </span>
                          <span>
                            {t(
                              "analytics.topProjectMeta",
                              "{{sessions}} sessions • {{messages}} msgs",
                              {
                                sessions: project.sessions,
                                messages: project.messages,
                              }
                            )}
                          </span>
                        </p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="font-mono text-sm font-bold text-foreground">
                        {formatNumber(project.tokens)}
                      </p>
                      <p className="text-sm text-muted-foreground">{t("analytics.tokens")}</p>
                    </div>
                  </div>
                );
              })}
            </div>
          </SectionCard>
        )}
      </div>

      {/* Most Used Tools */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5 mt-5">
        <SectionCard
          title={t("analytics.mostUsedToolsTitle")}
          icon={Wrench}
          colorVariant="amber"
        >
          <ToolUsageChart tools={globalSummary.most_used_tools} />
        </SectionCard>
      </div>
    </div>
  );
};

GlobalStatsView.displayName = "GlobalStatsView";
