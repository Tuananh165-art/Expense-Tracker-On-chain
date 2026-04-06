"use client";

import Link from "next/link";
import React, { useEffect, useMemo, useState } from "react";
import { getAccessToken } from "../../../lib/sdk";
import { useMe } from "../../../features/auth/useWalletAuth";
import { useCategories } from "../../../features/categories/useCategories";
import { useCreateExpense, useExpenseAuditLogs, useSearchExpenses, useUpdateExpenseStatus } from "../../../features/expenses/useExpenses";
import { Badge } from "../../../components/ui/badge";
import { Button, buttonVariants } from "../../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../../components/ui/card";
import { Input } from "../../../components/ui/input";
import { PageHeader } from "../../../components/page-header";
import { getHybridConfig } from "../../../lib/solana-wallet";
import { cn } from "../../../lib/utils";

export default function ExpensesPage() {
  const [isAuthed, setIsAuthed] = useState(false);
  const [expenseCategoryId, setExpenseCategoryId] = useState("");
  const [amountMinor, setAmountMinor] = useState(100000);
  const [currency, setCurrency] = useState("VND");
  const [expandedHistoryId, setExpandedHistoryId] = useState<string>("");
  const [searchStatus, setSearchStatus] = useState<"" | "pending" | "approved" | "rejected">("");
  const [searchCategoryId, setSearchCategoryId] = useState("");
  const [searchCurrency, setSearchCurrency] = useState("");
  const [searchKeyword, setSearchKeyword] = useState("");
  const [searchLimit, setSearchLimit] = useState(10);
  const [searchOffset, setSearchOffset] = useState(0);

  useEffect(() => {
    setIsAuthed(Boolean(getAccessToken()));
  }, []);

  const meQuery = useMe(isAuthed);
  const isAdmin = meQuery.data?.role === "admin";

  const searchParams = useMemo(
    () => ({
      status: searchStatus || undefined,
      category_id: searchCategoryId || undefined,
      currency: searchCurrency || undefined,
      q: searchKeyword || undefined,
      limit: searchLimit,
      offset: searchOffset,
    }),
    [searchStatus, searchCategoryId, searchCurrency, searchKeyword, searchLimit, searchOffset]
  );

  const expensesQuery = useSearchExpenses(searchParams, isAuthed);
  const categoriesQuery = useCategories(isAuthed);
  const createExpenseMutation = useCreateExpense();
  const updateExpenseStatusMutation = useUpdateExpenseStatus();
  const historyQuery = useExpenseAuditLogs(expandedHistoryId, isAuthed && Boolean(expandedHistoryId));

  useEffect(() => {
    setSearchOffset(0);
  }, [searchLimit]);

  const selectedCategory = (categoriesQuery.data ?? []).find((c) => c.id === expenseCategoryId);
  const hybridEnabled = getHybridConfig().enabled;

  return (
    <div className="space-y-8">
      <PageHeader title="Expenses" description="Tạo, lọc, duyệt và xem lịch sử expense." />

      <div className="surface-toolbar">
        <Button
          size="sm"
          onClick={async () => {
            await createExpenseMutation.mutateAsync({
              category_id: expenseCategoryId,
              amount_minor: amountMinor,
              currency,
            });
          }}
          disabled={!isAuthed || !expenseCategoryId || !currency || amountMinor <= 0}
        >
          Create Expense
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            setSearchOffset(0);
            setSearchStatus("");
            setSearchCategoryId("");
            setSearchCurrency("");
            setSearchKeyword("");
          }}
        >
          Clear Filters
        </Button>
        <Link href="/dashboard/reports" className={cn(buttonVariants({ variant: "ghost", size: "sm" }))}>
          Go to Reports
        </Link>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Create expense</CardTitle>
          <CardDescription>Tạo expense mới theo category.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          <div className="grid gap-2 md:grid-cols-4">
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
          {hybridEnabled && !selectedCategory?.name ? (
            <p className="text-xs text-danger">Hybrid mode requires a valid Category ID from committed categories.</p>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Filters</CardTitle>
          <CardDescription>Tìm kiếm và lọc danh sách expenses.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          <div className="grid gap-2 md:grid-cols-4">
            <Input
              placeholder="Filter category ID"
              value={searchCategoryId}
              onChange={(e) => {
                setSearchOffset(0);
                setSearchCategoryId(e.target.value);
              }}
            />
            <Input
              placeholder="Filter currency"
              value={searchCurrency}
              onChange={(e) => {
                setSearchOffset(0);
                setSearchCurrency(e.target.value);
              }}
            />
            <Input
              placeholder="Search keyword"
              value={searchKeyword}
              onChange={(e) => {
                setSearchOffset(0);
                setSearchKeyword(e.target.value);
              }}
            />
            <Input
              type="number"
              placeholder="Limit"
              value={searchLimit}
              onChange={(e) => setSearchLimit(Math.max(1, Number(e.target.value) || 10))}
            />
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              variant={searchStatus === "" ? "default" : "secondary"}
              onClick={() => {
                setSearchOffset(0);
                setSearchStatus("");
              }}
            >
              Status: All
            </Button>
            <Button
              variant={searchStatus === "pending" ? "default" : "secondary"}
              onClick={() => {
                setSearchOffset(0);
                setSearchStatus("pending");
              }}
            >
              Pending
            </Button>
            <Button
              variant={searchStatus === "approved" ? "default" : "secondary"}
              onClick={() => {
                setSearchOffset(0);
                setSearchStatus("approved");
              }}
            >
              Approved
            </Button>
            <Button
              variant={searchStatus === "rejected" ? "default" : "secondary"}
              onClick={() => {
                setSearchOffset(0);
                setSearchStatus("rejected");
              }}
            >
              Rejected
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">Active status filter: {searchStatus || "all"}</p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Expense list</CardTitle>
          <CardDescription>Danh sách theo bộ lọc hiện tại.</CardDescription>
        </CardHeader>
        <CardContent>
          <ul className="space-y-2 text-sm text-muted-foreground">
            {(expensesQuery.data?.items ?? []).map((e) => {
              const badgeVariant = e.status === "approved" ? "success" : e.status === "rejected" ? "danger" : "muted";
              const isHistoryExpanded = expandedHistoryId === e.id;
              return (
                <li key={e.id} className="surface-list-item">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-medium text-foreground">
                      {e.amount_minor} {e.currency}
                    </span>
                    <Badge variant={badgeVariant}>{e.status}</Badge>
                    <span>• {e.category_id}</span>
                  </div>

                  <div className="mt-2 flex flex-wrap gap-2">
                    {isAdmin && e.status === "pending" ? (
                      <>
                        <Button
                          variant="secondary"
                          disabled={updateExpenseStatusMutation.isPending}
                          onClick={() =>
                            updateExpenseStatusMutation.mutate({
                              expenseId: e.id,
                              status: "approved",
                            })
                          }
                        >
                          Approve
                        </Button>
                        <Button
                          variant="secondary"
                          disabled={updateExpenseStatusMutation.isPending}
                          onClick={() =>
                            updateExpenseStatusMutation.mutate({
                              expenseId: e.id,
                              status: "rejected",
                            })
                          }
                        >
                          Reject
                        </Button>
                      </>
                    ) : null}

                    {(isAdmin || meQuery.data?.role === "auditor") && (
                      <Button variant="secondary" onClick={() => setExpandedHistoryId(isHistoryExpanded ? "" : e.id)}>
                        {isHistoryExpanded ? "Hide History" : "Show History"}
                      </Button>
                    )}
                  </div>

                  {isHistoryExpanded ? (
                    <ul className="surface-subpanel mt-2 space-y-1 p-2 text-xs">
                      {(historyQuery.data ?? [])
                        .filter((x) =>
                          x.action === "expense.create" ||
                          x.action === "expense.approve" ||
                          x.action === "expense.reject"
                        )
                        .map((x) => (
                          <li key={x.id} className="flex flex-wrap items-center gap-2">
                            <span className="font-medium text-foreground">{x.action}</span>
                            <span>{new Date(x.created_at).toLocaleString()}</span>
                          </li>
                        ))}
                    </ul>
                  ) : null}
                </li>
              );
            })}
          </ul>

          <div className="mt-2 flex items-center gap-2 text-xs">
            <Button
              variant="secondary"
              disabled={searchOffset === 0}
              onClick={() => setSearchOffset((prev) => Math.max(0, prev - searchLimit))}
            >
              Prev
            </Button>
            <Button
              variant="secondary"
              disabled={!expensesQuery.data?.has_more}
              onClick={() => setSearchOffset((prev) => prev + searchLimit)}
            >
              Next
            </Button>
            <span>
              Total: {expensesQuery.data?.total ?? 0} • Offset: {searchOffset} • Limit: {searchLimit}
            </span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
