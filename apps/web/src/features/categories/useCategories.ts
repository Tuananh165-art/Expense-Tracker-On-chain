"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../../lib/sdk";
import {
  connectSolanaWallet,
  getHybridConfig,
  getSolanaProvider,
  getWalletBalanceLamports,
  requestSolAirdrop,
  sendCreateCategoryTx,
} from "../../lib/solana-wallet";

export const categoriesKey = ["categories"] as const;

export interface WalletFundingStatus {
  status: "funded" | "airdropped";
  walletAddress: string;
  rpcUrl: string;
  lamportsBefore: number;
  lamportsAfter: number;
}

async function retryCategoryCommit(txHash: string, categoryName: string, maxAttempts = 8) {
  let lastError: unknown;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      return await apiClient.commitCategoryOnchain({ tx_hash: txHash, category_name: categoryName });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      lastError = error;
      if (!message.includes("transaction not found at selected commitment") || attempt === maxAttempts) {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 500 * attempt));
    }
  }
  throw lastError instanceof Error ? lastError : new Error("onchain category commit failed");
}

export function useCategories(enabled = true) {
  return useQuery({
    queryKey: categoriesKey,
    queryFn: () => apiClient.listCategories(),
    enabled,
  });
}

export function useCheckWalletSolOrRequestAirdrop() {
  return useMutation<WalletFundingStatus>({
    mutationFn: async () => {
      const hybrid = getHybridConfig();
      if (!hybrid.enabled) {
        throw new Error("Hybrid mode is disabled");
      }

      const provider = getSolanaProvider();
      if (!provider) {
        throw new Error("Solana wallet not found");
      }

      const walletAddress = await connectSolanaWallet(provider);
      const { lamports: lamportsBefore, rpcUrl } = await getWalletBalanceLamports(walletAddress);
      if (lamportsBefore > 0) {
        return {
          status: "funded",
          walletAddress,
          rpcUrl,
          lamportsBefore,
          lamportsAfter: lamportsBefore,
        };
      }

      const { lamportsAfter } = await requestSolAirdrop(walletAddress);
      return {
        status: "airdropped",
        walletAddress,
        rpcUrl,
        lamportsBefore,
        lamportsAfter,
      };
    },
  });
}

export function useCreateCategory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (name: string) => {
      const categoryName = name.trim();
      if (!categoryName) {
        throw new Error("Category name is required");
      }

      const hybrid = getHybridConfig();
      if (!hybrid.enabled) {
        return apiClient.createCategory({ name: categoryName });
      }

      const provider = getSolanaProvider();
      if (!provider) {
        throw new Error("Solana wallet not found");
      }

      const walletAddress = await connectSolanaWallet(provider);
      const { txHash } = await sendCreateCategoryTx(provider, walletAddress, categoryName);
      return retryCategoryCommit(txHash, categoryName);
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: categoriesKey }),
  });
}
