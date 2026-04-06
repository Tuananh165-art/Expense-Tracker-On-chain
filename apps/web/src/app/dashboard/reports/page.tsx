"use client";

import Link from "next/link";
import React, { useEffect, useMemo, useState } from "react";
import { getAccessToken } from "../../../lib/sdk";
import { useMonthlyReport } from "../../../features/reports/useReports";
import { Button, buttonVariants } from "../../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../../components/ui/card";
import { Input } from "../../../components/ui/input";
import { PageHeader } from "../../../components/page-header";
import { cn } from "../../../lib/utils";

export default function ReportsPage() {
  const [isAuthed, setIsAuthed] = useState(false);
  const now = useMemo(() => new Date(), []);
  const [reportMonth, setReportMonth] = useState(now.getMonth() + 1);
  const [reportYear, setReportYear] = useState(now.getFullYear());
  const [reportTimezone, setReportTimezone] = useState(Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC");

  useEffect(() => {
    setIsAuthed(Boolean(getAccessToken()));
  }, []);

  const monthlyReportQuery = useMonthlyReport(
    { month: reportMonth, year: reportYear, timezone: reportTimezone, top_n: 5 },
    isAuthed
  );

  return (
    <div className="space-y-8">
      <PageHeader title="Monthly reports" description="Báo cáo chi tiêu theo tháng, category, ngày và top spending." />

      <div className="surface-toolbar">
        <Button
          size="sm"
          onClick={() => {
            const current = new Date();
            setReportMonth(current.getMonth() + 1);
            setReportYear(current.getFullYear());
            setReportTimezone(Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC");
          }}
        >
          Reset Current Month
        </Button>
        <Link href="/dashboard/expenses" className={cn(buttonVariants({ variant: "secondary", size: "sm" }))}>
          Go to Expenses
        </Link>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Report filters</CardTitle>
          <CardDescription>Chọn month/year/timezone để xem dữ liệu.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-2 md:grid-cols-3">
          <Input
            type="number"
            placeholder="Month"
            value={reportMonth}
            onChange={(e) => setReportMonth(Number(e.target.value))}
          />
          <Input
            type="number"
            placeholder="Year"
            value={reportYear}
            onChange={(e) => setReportYear(Number(e.target.value))}
          />
          <Input
            placeholder="Timezone (e.g. Asia/Ho_Chi_Minh)"
            value={reportTimezone}
            onChange={(e) => setReportTimezone(e.target.value)}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Summary</CardTitle>
          <CardDescription>Tổng quan tháng đã chọn.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-base font-medium">
            Total minor: <span className="text-accent">{monthlyReportQuery.data?.total_amount_minor ?? 0}</span>
          </p>
          <p className="text-xs text-muted-foreground">
            Period: {monthlyReportQuery.data?.period?.from_utc} → {monthlyReportQuery.data?.period?.to_utc}
          </p>

          <p className="text-sm font-semibold text-foreground">By Category</p>
          <ul className="space-y-2 text-sm text-muted-foreground">
            {(monthlyReportQuery.data?.by_category ?? []).map((x) => (
              <li key={x.category_id} className="surface-list-item">
                {x.category_id}: <span className="font-medium text-foreground">{x.total_amount_minor}</span>
              </li>
            ))}
          </ul>

          <p className="text-sm font-semibold text-foreground">By Day</p>
          <ul className="space-y-2 text-sm text-muted-foreground">
            {(monthlyReportQuery.data?.by_day ?? []).map((x) => (
              <li key={x.day} className="surface-list-item">
                {x.day}: <span className="font-medium text-foreground">{x.total_amount_minor}</span>
              </li>
            ))}
          </ul>

          <p className="text-sm font-semibold text-foreground">Top Spending</p>
          <ul className="space-y-2 text-sm text-muted-foreground">
            {(monthlyReportQuery.data?.top_spending ?? []).map((x) => (
              <li key={x.id} className="surface-list-item">
                {x.amount_minor} {x.currency} • {x.status} • {x.category_id}
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
