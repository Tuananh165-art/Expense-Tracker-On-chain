"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../../lib/sdk";

export const categoriesKey = ["categories"] as const;

export function useCategories(enabled = true) {
  return useQuery({
    queryKey: categoriesKey,
    queryFn: () => apiClient.listCategories(),
    enabled,
  });
}

export function useCreateCategory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => apiClient.createCategory({ name }),
    onSuccess: () => qc.invalidateQueries({ queryKey: categoriesKey }),
  });
}
