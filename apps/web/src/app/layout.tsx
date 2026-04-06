import type { Metadata } from "next";
import React from "react";
import { Providers } from "../components/providers";
import "./globals.css";

export const metadata: Metadata = {
  title: "Expense Tracker On-chain",
  description: "Enterprise baseline for transparent personal expense tracking",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
