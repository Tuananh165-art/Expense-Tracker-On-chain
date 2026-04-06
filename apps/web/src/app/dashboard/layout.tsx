"use client";

import React, { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { AppShell } from "../../components/app-shell";
import { getAccessToken } from "../../lib/sdk";

export default function DashboardLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  const router = useRouter();
  const [isCheckingAuth, setIsCheckingAuth] = useState(true);
  const [isAuthed, setIsAuthed] = useState(false);

  useEffect(() => {
    const hasToken = Boolean(getAccessToken());
    setIsAuthed(hasToken);
    if (!hasToken) router.replace("/sign-in");
    setIsCheckingAuth(false);
  }, [router]);

  if (isCheckingAuth || !isAuthed) {
    return (
      <main className="mx-auto flex min-h-screen w-full max-w-6xl items-center justify-center px-4 md:px-8">
        <div className="w-full max-w-xl animate-pulse rounded-xl border border-border/70 bg-surface-1/80 p-5 shadow-soft">
          <div className="mb-3 h-4 w-36 rounded bg-surface-3/80" />
          <div className="mb-2 h-9 w-2/3 rounded bg-surface-2/85" />
          <div className="mb-5 h-4 w-full rounded bg-surface-2/70" />
          <div className="h-10 w-44 rounded-md bg-primary/25" />
        </div>
      </main>
    );
  }

  return <AppShell>{children}</AppShell>;
}
