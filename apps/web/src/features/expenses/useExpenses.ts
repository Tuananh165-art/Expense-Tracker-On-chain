"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../../lib/sdk";
import {
  connectSolanaWallet,
  deriveCategoryPda,
  getHybridConfig,
  getSolanaProvider,
  makeExpenseIdOnchain,
  sendCreateExpenseTx,
  sendUpdateExpenseStatusTx,
} from "../../lib/solana-wallet";
import { monthlyReportKey } from "../reports/useReports";

export const expensesKey = ["expenses"] as const;

export type SearchExpenseParams = {
  status?: "pending" | "approved" | "rejected";
  category_id?: string;
  currency?: string;
  from?: string;
  to?: string;
  q?: string;
  limit?: number;
  offset?: number;
};

type CreateExpenseMutationInput = {
  category_id: string;
  amount_minor: number;
  currency: string;
};

export function expenseAuditKey(expenseId: string) {
  return ["audit", "expense", expenseId] as const;
}

export function expenseSearchKey(params: SearchExpenseParams) {
  return [...expensesKey, "search", params] as const;
}

export function useExpenses(enabled = true) {
  return useQuery({
    queryKey: expensesKey,
    queryFn: () => apiClient.listExpenses(),
    enabled,
  });
}

export function useSearchExpenses(params: SearchExpenseParams, enabled = true) {
  return useQuery({
    queryKey: expenseSearchKey(params),
    queryFn: () => apiClient.searchExpenses(params),
    enabled,
  });
}

export function useExpenseAuditLogs(expenseId: string, enabled = true) {
  return useQuery({
    queryKey: expenseAuditKey(expenseId),
    queryFn: () => apiClient.listExpenseHistory(expenseId, { limit: 100 }),
    enabled,
  });
}

async function retryExpenseCreateCommit(
  payload: {
    tx_hash: string;
    expense_id_onchain: number;
    category_pda: string;
    amount_minor: number;
    currency: string;
  },
  maxAttempts = 8
) {
  let lastError: unknown;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      return await apiClient.commitExpenseCreateOnchain(payload);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      lastError = error;
      if (!message.includes("transaction not found at selected commitment") || attempt === maxAttempts) {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 500 * attempt));
    }
  }
  throw lastError instanceof Error ? lastError : new Error("onchain expense create commit failed");
}

export function useCreateExpense() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (payload: CreateExpenseMutationInput) => {
      const hybrid = getHybridConfig();
      if (!hybrid.enabled) {
        const idempotencyKey = crypto.randomUUID();
        return apiClient.createExpense(payload, idempotencyKey);
      }


      const provider = getSolanaProvider();
      if (!provider) {
        throw new Error("Solana wallet not found");
      }

      const walletAddress = await connectSolanaWallet(provider);
      const categoryId = payload.category_id.trim();
      const categories = await apiClient.listCategories();
      const category = categories.find((item) => item.id === categoryId);
      if (!category) {
        throw new Error("Category ID not found. Use a committed category ID from Category list.");
      }

      const categoryPda = deriveCategoryPda(walletAddress, category.name, hybrid.programId);
      const expenseIdOnchain = makeExpenseIdOnchain();

      const { txHash } = await sendCreateExpenseTx(
        provider,
        walletAddress,
        categoryPda,
        expenseIdOnchain,
        payload.amount_minor
      );

      return retryExpenseCreateCommit({
        tx_hash: txHash,
        expense_id_onchain: expenseIdOnchain,
        category_pda: categoryPda,
        amount_minor: payload.amount_minor,
        currency: payload.currency,
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: expensesKey });
      qc.invalidateQueries({ queryKey: monthlyReportKey });
    },
  });
}

export function useUpdateExpenseStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({
      expenseId,
      status,
      reason,
    }: {
      expenseId: string;
      status: "approved" | "rejected";
      reason?: string;
    }) => {
      const hybrid = getHybridConfig();
      if (!hybrid.enabled) {
        const idempotencyKey = crypto.randomUUID();
        return apiClient.updateExpenseStatus(expenseId, { status, reason }, idempotencyKey);
      }

      const history = await apiClient.listExpenseHistory(expenseId, { limit: 100 });
      const createLog = history.find((item) => item.action === "expense.create");
      const meta = (createLog?.metadata ?? {}) as Record<string, unknown>;
      const onchainExpensePda =
        typeof meta.onchain_expense_pda === "string" ? meta.onchain_expense_pda : undefined;
      if (!onchainExpensePda) {
        throw new Error("onchain expense metadata not found for status update");
      }

      const provider = getSolanaProvider();
      if (!provider) {
        throw new Error("Solana wallet not found");
      }

      const connectedWallet = await connectSolanaWallet(provider);
      const txHash = await sendUpdateExpenseStatusTx(provider, connectedWallet, onchainExpensePda, status);
      return apiClient.commitExpenseStatusOnchain(expenseId, {
        tx_hash: txHash,
        to_status: status,
        reason,
      });
    },
    onSuccess: (_, vars) => {
      qc.invalidateQueries({ queryKey: expensesKey });
      qc.invalidateQueries({ queryKey: monthlyReportKey });
      qc.invalidateQueries({ queryKey: expenseAuditKey(vars.expenseId) });
    },
  });
}
