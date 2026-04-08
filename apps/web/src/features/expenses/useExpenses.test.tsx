import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createQueryClientWrapper } from "../../test/queryClientTestUtils";
import { apiClient } from "../../lib/sdk";
import {
  expenseAuditKey,
  expensesKey,
  expenseSearchKey,
  useCreateExpense,
  useSearchExpenses,
  useUpdateExpenseStatus,
} from "./useExpenses";
import { monthlyReportKey } from "../reports/useReports";

vi.mock("../../lib/sdk", () => ({
  apiClient: {
    listCategories: vi.fn(),
    searchExpenses: vi.fn(),
    createExpense: vi.fn(),
    updateExpenseStatus: vi.fn(),
    listExpenseHistory: vi.fn(),
    commitExpenseCreateOnchain: vi.fn(),
    commitExpenseStatusOnchain: vi.fn(),
  },
}));

vi.mock("../../lib/solana-wallet", () => ({
  connectSolanaWallet: vi.fn(),
  deriveCategoryPda: vi.fn(),
  getHybridConfig: vi.fn(() => ({ enabled: false, programId: "test-program" })),
  getSolanaProvider: vi.fn(),
  makeExpenseIdOnchain: vi.fn(() => 42),
  sendCreateExpenseTx: vi.fn(),
  sendUpdateExpenseStatusTx: vi.fn(),
}));

describe("expenses hooks smoke", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("useSearchExpenses calls search endpoint", async () => {
    apiClient.searchExpenses.mockResolvedValueOnce({
      items: [],
      total: 0,
      limit: 10,
      offset: 0,
      has_more: false,
    });

    const params = { status: "pending" as const, limit: 10, offset: 0 };
    const { Wrapper } = createQueryClientWrapper();
    const { result } = renderHook(() => useSearchExpenses(params, true), { wrapper: Wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(apiClient.searchExpenses).toHaveBeenCalledWith(params);
  });

  it("useCreateExpense (hybrid off) calls createExpense with idempotency key", async () => {
    apiClient.createExpense.mockResolvedValueOnce({
      id: "e1",
      owner_user_id: "u1",
      category_id: "c1",
      amount_minor: 1000,
      currency: "VND",
      status: "pending",
      tx_hash: null,
      occurred_at: "2026-01-01T00:00:00Z",
      created_at: "2026-01-01T00:00:00Z",
    });

    const { Wrapper } = createQueryClientWrapper();
    const { result } = renderHook(() => useCreateExpense(), { wrapper: Wrapper });

    await result.current.mutateAsync({ category_id: "c1", amount_minor: 1000, currency: "VND" });

    expect(apiClient.createExpense).toHaveBeenCalledTimes(1);
    expect(apiClient.createExpense.mock.calls[0][0]).toEqual({
      category_id: "c1",
      amount_minor: 1000,
      currency: "VND",
    });
    expect(typeof apiClient.createExpense.mock.calls[0][1]).toBe("string");
  });

  it("useUpdateExpenseStatus invalidates related queries", async () => {
    apiClient.updateExpenseStatus.mockResolvedValueOnce({
      id: "e1",
      owner_user_id: "u1",
      category_id: "c1",
      amount_minor: 1000,
      currency: "VND",
      status: "approved",
      tx_hash: null,
      occurred_at: "2026-01-01T00:00:00Z",
      created_at: "2026-01-01T00:00:00Z",
    });

    const { client, Wrapper } = createQueryClientWrapper();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");

    const { result } = renderHook(() => useUpdateExpenseStatus(), { wrapper: Wrapper });

    await result.current.mutateAsync({ expenseId: "e1", status: "approved" });

    expect(apiClient.updateExpenseStatus).toHaveBeenCalledTimes(1);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: expensesKey });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: monthlyReportKey });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: expenseAuditKey("e1") });
  });

  it("expenseSearchKey remains stable shape", () => {
    expect(expenseSearchKey({ status: "pending" })).toEqual(["expenses", "search", { status: "pending" }]);
  });
});
