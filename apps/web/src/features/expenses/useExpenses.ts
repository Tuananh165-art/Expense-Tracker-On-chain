"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../../lib/sdk";

export const expensesKey = ["expenses"] as const;

export function useExpenses(enabled = true) {
  return useQuery({
    queryKey: expensesKey,
    queryFn: () => apiClient.listExpenses(),
    enabled,
  });
}

export function useCreateExpense() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (payload: { category_id: string; amount_minor: number; currency: string }) => {
      const idempotencyKey = crypto.randomUUID();
      return apiClient.createExpense(payload, idempotencyKey);
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: expensesKey }),
  });
}
