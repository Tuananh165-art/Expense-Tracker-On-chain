"use client";

import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../../lib/sdk";

export const monthlyReportKey = ["reports", "monthly"] as const;

export function useMonthlyReport(enabled = true) {
  return useQuery({
    queryKey: monthlyReportKey,
    queryFn: () => apiClient.monthlyReport(),
    enabled,
  });
}
