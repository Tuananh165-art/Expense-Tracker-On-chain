"use client";

import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../../lib/sdk";

export type MonthlyReportParams = {
  month?: number;
  year?: number;
  timezone?: string;
  top_n?: number;
};

export const monthlyReportKey = ["reports", "monthly"] as const;

export function monthlyReportQueryKey(params: MonthlyReportParams) {
  return [...monthlyReportKey, params] as const;
}

export function useMonthlyReport(params: MonthlyReportParams, enabled = true) {
  return useQuery({
    queryKey: monthlyReportQueryKey(params),
    queryFn: () => apiClient.monthlyReport(params),
    enabled,
  });
}
