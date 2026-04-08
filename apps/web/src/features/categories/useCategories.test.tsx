import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createQueryClientWrapper } from "../../test/queryClientTestUtils";
import { apiClient } from "../../lib/sdk";
import { useCategories, useCreateCategory } from "./useCategories";

vi.mock("../../lib/sdk", () => ({
  apiClient: {
    listCategories: vi.fn(),
    createCategory: vi.fn(),
    commitCategoryOnchain: vi.fn(),
  },
}));

vi.mock("../../lib/solana-wallet", () => ({
  connectSolanaWallet: vi.fn(),
  getHybridConfig: vi.fn(() => ({ enabled: false })),
  getSolanaProvider: vi.fn(),
  getWalletBalanceLamports: vi.fn(),
  requestSolAirdrop: vi.fn(),
  sendCreateCategoryTx: vi.fn(),
}));

describe("categories hooks smoke", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("useCategories calls listCategories", async () => {
    apiClient.listCategories.mockResolvedValueOnce([
      { id: "c1", owner_user_id: "u1", name: "food", created_at: "2026-01-01T00:00:00Z" },
    ]);

    const { Wrapper } = createQueryClientWrapper();
    const { result } = renderHook(() => useCategories(true), { wrapper: Wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(apiClient.listCategories).toHaveBeenCalledTimes(1);
    expect(result.current.data?.[0]?.name).toBe("food");
  });

  it("useCreateCategory (hybrid off) calls createCategory", async () => {
    apiClient.createCategory.mockResolvedValueOnce({
      id: "c2",
      owner_user_id: "u1",
      name: "travel",
      created_at: "2026-01-01T00:00:00Z",
    });

    const { Wrapper } = createQueryClientWrapper();
    const { result } = renderHook(() => useCreateCategory(), { wrapper: Wrapper });

    await result.current.mutateAsync(" travel ");

    expect(apiClient.createCategory).toHaveBeenCalledWith({ name: "travel" });
  });

  it("useCreateCategory rejects empty name", async () => {
    const { Wrapper } = createQueryClientWrapper();
    const { result } = renderHook(() => useCreateCategory(), { wrapper: Wrapper });

    await expect(result.current.mutateAsync("   ")).rejects.toThrow("Category name is required");
    expect(apiClient.createCategory).not.toHaveBeenCalled();
  });
});
