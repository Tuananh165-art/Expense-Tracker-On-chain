"use client";

import React, { useEffect, useState } from "react";
import { motion, useReducedMotion } from "framer-motion";
import { getAccessToken } from "../lib/sdk";
import { useWalletAuth } from "../features/auth/useWalletAuth";
import { useCategories, useCreateCategory } from "../features/categories/useCategories";
import { useCreateExpense, useExpenses } from "../features/expenses/useExpenses";
import { useMonthlyReport } from "../features/reports/useReports";
import { connectSolanaWallet, getSolanaProvider, signChallengeMessage } from "../lib/solana-wallet";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/ui/card";
import { Input } from "../components/ui/input";

const spring = { type: "spring", stiffness: 110, damping: 18 } as const;

export default function HomePage() {
  const [connectedWallet, setConnectedWallet] = useState<string>("");
  const [authError, setAuthError] = useState<string>("");

  const [categoryName, setCategoryName] = useState("");
  const [expenseCategoryId, setExpenseCategoryId] = useState("");
  const [amountMinor, setAmountMinor] = useState(100000);
  const [currency, setCurrency] = useState("VND");

  const [isAuthed, setIsAuthed] = useState(false);
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    setIsAuthed(Boolean(getAccessToken()));
  }, []);

  const { challengeMutation, verifyMutation } = useWalletAuth();

  const categoriesQuery = useCategories(isAuthed);
  const createCategoryMutation = useCreateCategory();

  const expensesQuery = useExpenses(isAuthed);
  const createExpenseMutation = useCreateExpense();

  const monthlyReportQuery = useMonthlyReport(isAuthed);

  const sectionMotion = reducedMotion
    ? { initial: false, animate: { opacity: 1 } }
    : { initial: { opacity: 0, y: 14 }, animate: { opacity: 1, y: 0 }, transition: spring };

  const isAuthLoading = challengeMutation.isPending || verifyMutation.isPending;

  async function handleConnectWallet() {
    setAuthError("");
    const provider = getSolanaProvider();
    if (!provider) {
      setAuthError("Không tìm thấy ví Solana (Phantom/Solflare). Hãy cài extension ví trước.");
      return;
    }

    try {
      const wallet = await connectSolanaWallet(provider);
      setConnectedWallet(wallet);
    } catch {
      setAuthError("Kết nối ví thất bại.");
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
      const wallet = connectedWallet || (await connectSolanaWallet(provider));
      if (!connectedWallet) setConnectedWallet(wallet);

      const challenge = await challengeMutation.mutateAsync(wallet);
      const signature = await signChallengeMessage(provider, challenge.message);

      await verifyMutation.mutateAsync({
        challengeId: challenge.challenge_id,
        walletAddress: wallet,
        signature,
      });
      setIsAuthed(true);
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
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-6 px-4 py-10 md:px-8">
      <motion.section
        {...sectionMotion}
        className="rounded-2xl border border-border/70 bg-card/90 p-6 shadow-[0_30px_70px_-45px_rgba(31,38,135,0.8)] backdrop-blur"
      >
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-3xl font-bold tracking-tight">Expense Tracker On-chain</h1>
          <Badge variant="success">Solana Localnet Ready</Badge>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          Dashboard hiện đại cho wallet login, categories, expenses và monthly report.
        </p>
      </motion.section>

      <motion.div {...sectionMotion} className="grid gap-5 lg:grid-cols-2">
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>* Session</CardTitle>
            <CardDescription>Đăng nhập 1-click bằng ví Solana để nhận JWT phiên làm việc.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex flex-wrap gap-2">
              <Button variant="secondary" onClick={handleConnectWallet} disabled={isAuthLoading}>
                {connectedWallet ? "Wallet Connected" : "Connect Wallet"}
              </Button>
              <Button onClick={handleSignIn} disabled={isAuthLoading || !connectedWallet}>
                {isAuthLoading ? "Signing In..." : "Sign In With Wallet"}
              </Button>
            </div>
            <p className="text-sm">
              Authenticated: <span className="font-semibold">{isAuthed ? "Yes" : "No"}</span>
            </p>
            {isAuthed ? (
              <p className="text-sm text-green-300">Login success</p>
            ) : (
              <p className="text-sm text-muted-foreground">Chưa đăng nhập</p>
            )}
            {authError ? <p className="text-sm text-red-300">{authError}</p> : null}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>* Categories</CardTitle>
            <CardDescription>Tạo category mới và xem danh sách hiện có.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex gap-2">
              <Input
                placeholder="Category name"
                value={categoryName}
                onChange={(e) => setCategoryName(e.target.value)}
              />
              <Button
                variant="secondary"
                onClick={async () => {
                  await createCategoryMutation.mutateAsync(categoryName);
                  setCategoryName("");
                }}
                disabled={!isAuthed || !categoryName}
              >
                Create
              </Button>
            </div>
            <ul className="space-y-1 text-sm text-muted-foreground">
              {(categoriesQuery.data ?? []).map((c) => (
                <li key={c.id} className="rounded-md border border-border/60 bg-background/30 px-3 py-2">
                  <span className="font-medium text-foreground">{c.name}</span> • {c.id}
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>

        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>* Expenses</CardTitle>
            <CardDescription>Tạo expense mới và xem trạng thái xử lý giao dịch.</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-3 lg:grid-cols-2">
            <div className="grid gap-2">
              <Input
                placeholder="Category ID"
                value={expenseCategoryId}
                onChange={(e) => setExpenseCategoryId(e.target.value)}
              />
              <Input
                type="number"
                placeholder="Amount minor"
                value={amountMinor}
                onChange={(e) => setAmountMinor(Number(e.target.value))}
              />
              <Input placeholder="Currency" value={currency} onChange={(e) => setCurrency(e.target.value)} />
              <Button
                disabled={!isAuthed || !expenseCategoryId || !currency || amountMinor <= 0}
                onClick={async () => {
                  await createExpenseMutation.mutateAsync({
                    category_id: expenseCategoryId,
                    amount_minor: amountMinor,
                    currency,
                  });
                }}
              >
                Create Expense
              </Button>
            </div>
            <ul className="space-y-2 text-sm text-muted-foreground">
              {(expensesQuery.data ?? []).map((e) => (
                <li key={e.id} className="rounded-md border border-border/60 bg-background/30 px-3 py-2">
                  <span className="font-medium text-foreground">
                    {e.amount_minor} {e.currency}
                  </span>{" "}
                  • {e.status} • {e.category_id}
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>

        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>* Monthly Report</CardTitle>
            <CardDescription>Tổng hợp chi tiêu theo tháng và theo category.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-base font-medium">
              Total minor: <span className="text-accent">{monthlyReportQuery.data?.total_amount_minor ?? 0}</span>
            </p>
            <ul className="space-y-2 text-sm text-muted-foreground">
              {(monthlyReportQuery.data?.by_category ?? []).map((x) => (
                <li key={x.category_id} className="rounded-md border border-border/60 bg-background/30 px-3 py-2">
                  {x.category_id}: <span className="font-medium text-foreground">{x.total_amount_minor}</span>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      </motion.div>
    </main>
  );
}
