"use client";

import { useMutation } from "@tanstack/react-query";
import { apiClient, setAccessToken } from "../../lib/sdk";

export function useWalletAuth() {
  const challengeMutation = useMutation({
    mutationFn: async (walletAddress: string) => {
      return apiClient.challenge({ wallet_address: walletAddress });
    },
  });

  const verifyMutation = useMutation({
    mutationFn: async ({
      challengeId,
      walletAddress,
      signature,
    }: {
      challengeId: string;
      walletAddress: string;
      signature: string;
    }) => {
      const result = await apiClient.verify({
        challenge_id: challengeId,
        wallet_address: walletAddress,
        signature,
      });
      setAccessToken(result.access_token);
      return result;
    },
  });

  return { challengeMutation, verifyMutation };
}
