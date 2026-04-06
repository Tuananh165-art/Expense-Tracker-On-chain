import React from "react";
import { AppNav } from "./app-nav";

export function AppShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen">
      <AppNav />
      <main className="mx-auto w-full max-w-6xl px-4 py-10 md:px-8 md:py-12">{children}</main>
    </div>
  );
}
