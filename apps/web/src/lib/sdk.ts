"use client";

import { ExpenseApiClient } from "@expense/sdk";

export const tokenStorageKey = "expense_tracker_access_token";

export function getAccessToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(tokenStorageKey);
}

export function setAccessToken(token: string) {
  if (typeof window === "undefined") return;
  localStorage.setItem(tokenStorageKey, token);
}

export const apiClient = new ExpenseApiClient(
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080",
  getAccessToken
);
