"use client";

import React, { useEffect } from "react";
import { useRouter } from "next/navigation";
import { getAccessToken } from "../lib/sdk";

export default function EntryPage() {
  const router = useRouter();

  useEffect(() => {
    router.replace(getAccessToken() ? "/dashboard" : "/sign-in");
  }, [router]);

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl items-center justify-center px-4 md:px-8">
      <div className="w-full max-w-xl animate-pulse rounded-xl border border-border/70 bg-surface-1/80 p-5 shadow-soft">
        <div className="mb-3 h-4 w-32 rounded bg-surface-3/80" />
        <div className="mb-2 h-9 w-3/4 rounded bg-surface-2/85" />
        <div className="mb-5 h-4 w-full rounded bg-surface-2/70" />
        <div className="h-10 w-40 rounded-md bg-primary/25" />
      </div>
    </main>
  );
}
