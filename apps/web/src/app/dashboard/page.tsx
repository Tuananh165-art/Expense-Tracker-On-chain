"use client";

import React, { useEffect, useMemo, useState } from "react";
import { getAccessToken } from "../../lib/sdk";
import { useMe } from "../../features/auth/useWalletAuth";
import { useCategories } from "../../features/categories/useCategories";
import { useSearchExpenses } from "../../features/expenses/useExpenses";
import { useMonthlyReport } from "../../features/reports/useReports";
import { Badge } from "../../components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../components/ui/card";
import { PageHeader } from "../../components/page-header";

export default function DashboardOverviewPage() {
  const [isAuthed, setIsAuthed] = useState(false);
  const now = useMemo(() => new Date(), []);

  useEffect(() => {
    setIsAuthed(Boolean(getAccessToken()));
  }, []);

  const meQuery = useMe(isAuthed);
  const categoriesQuery = useCategories(isAuthed);
  const expensesQuery = useSearchExpenses({ limit: 10, offset: 0 }, isAuthed);
  const reportQuery = useMonthlyReport(
    {
      month: now.getMonth() + 1,
      year: now.getFullYear(),
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
      top_n: 5,
    },
    isAuthed
  );

  return (
    <div className="space-y-8">
      <PageHeader
        mode="hero"
        title="Dashboard"
        description="Workspace tập trung cho wallet auth, expense workflow, auditability và monthly reporting."
      />

      <div className="surface-subpanel flex flex-wrap gap-2 p-3">
        <Badge variant="success">Wallet signature auth</Badge>
        <Badge variant="default">RBAC enforcement</Badge>
        <Badge variant="muted">Immutable audit trail</Badge>
        <Badge variant="default">Idempotent writes</Badge>
      </div>


      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Auth</CardTitle>
            <CardDescription>Phiên đăng nhập</CardDescription>
          </CardHeader>
          <CardContent>
            <Badge variant={isAuthed ? "success" : "muted"}>{isAuthed ? "Authenticated" : "Guest"}</Badge>
            {meQuery.data?.role ? <p className="mt-2 text-xs text-muted-foreground">Role: {meQuery.data.role}</p> : null}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Categories</CardTitle>
            <CardDescription>Số lượng hiện có</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-semibold">{categoriesQuery.data?.length ?? 0}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Expenses</CardTitle>
            <CardDescription>Kết quả tìm kiếm gần nhất</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-semibold">{expensesQuery.data?.total ?? 0}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Monthly Total</CardTitle>
            <CardDescription>Tháng hiện tại</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-semibold">{reportQuery.data?.total_amount_minor ?? 0}</p>
          </CardContent>
        </Card>
      </div>

    </div>
  );
}
