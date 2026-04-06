"use client";

import React, { useState } from "react";
import { useRouter } from "next/navigation";
import { useWalletAuth } from "../../features/auth/useWalletAuth";
import { connectSolanaWallet, getSolanaProvider, signChallengeMessage } from "../../lib/solana-wallet";
import { Button, buttonVariants } from "../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../components/ui/card";
import { cn } from "../../lib/utils";

export default function SignInPage() {
  const router = useRouter();
  const [connectedWallet, setConnectedWallet] = useState("");
  const [authError, setAuthError] = useState("");

  const { challengeMutation, verifyMutation } = useWalletAuth();
  const isAuthLoading = challengeMutation.isPending || verifyMutation.isPending;

  async function connectWalletIfNeeded(): Promise<string | null> {
    const provider = getSolanaProvider();
    if (!provider) {
      setAuthError("Không tìm thấy ví Solana (Phantom/Solflare). Hãy cài extension ví trước.");
      return null;
    }

    if (connectedWallet) return connectedWallet;

    try {
      const wallet = await connectSolanaWallet(provider);
      setConnectedWallet(wallet);
      return wallet;
    } catch {
      setAuthError("Kết nối ví thất bại.");
      return null;
    }
  }

  async function handleSignIn() {
    setAuthError("");
    const provider = getSolanaProvider();
    if (!provider) {
      setAuthError("Không tìm thấy ví Solana (Phantom/Solflare).");
      return;
    }

    try {
      const wallet = await connectWalletIfNeeded();
      if (!wallet) return;

      const challenge = await challengeMutation.mutateAsync(wallet);
      const signature = await signChallengeMessage(provider, challenge.message);

      await verifyMutation.mutateAsync({
        challengeId: challenge.challenge_id,
        walletAddress: wallet,
        signature,
      });
      router.replace("/dashboard");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Đăng nhập ví thất bại. Hãy thử lại.";
      if (message.toLowerCase().includes("does not support signmessage")) {
        setAuthError("Ví hiện tại không hỗ trợ ký message. Hãy dùng Phantom/Solflare và bật quyền ký.");
      } else {
        setAuthError(`Đăng nhập ví thất bại: ${message}`);
      }
    }
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl items-center justify-center px-4 py-10 md:px-8 md:py-12">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <CardTitle>Sign In With Wallet</CardTitle>
          <CardDescription>Đăng nhập bằng ví Solana để vào dashboard.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Button
            onClick={handleSignIn}
            disabled={isAuthLoading}
            className="w-full"
            title={!connectedWallet ? "Sign in sẽ tự connect wallet nếu chưa kết nối." : undefined}
          >
            {isAuthLoading ? "Signing In..." : "Sign In With Wallet"}
          </Button>

          {!connectedWallet ? (
            <p className="text-xs text-muted-foreground">
              Tip: Nhấn <span className="font-medium text-foreground">Sign In With Wallet</span> để auto-connect ví rồi ký message.
            </p>
          ) : null}

          {authError ? <p className="text-sm text-red-300">{authError}</p> : null}

          <div className="pt-2">
            <button
              type="button"
              className={cn(buttonVariants({ variant: "ghost", size: "sm" }))}
              onClick={() => router.replace("/")}
            >
              Back
            </button>
          </div>
        </CardContent>
      </Card>
    </main>
  );
}
