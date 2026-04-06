"use client";

import { ExpenseApiClient } from "@expense/sdk";

export const tokenStorageKey = "expense_tracker_access_token";
const sessionCookieKey = "expense_tracker_session";

function setSessionCookie(enabled: boolean) {
  if (typeof document === "undefined") return;
  const maxAge = enabled ? 60 * 60 * 24 * 14 : 0;
  document.cookie = `${sessionCookieKey}=${enabled ? "1" : ""}; path=/; max-age=${maxAge}; samesite=lax`;
}

export function getAccessToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(tokenStorageKey);
}

export function setAccessToken(token: string) {
  if (typeof window === "undefined") return;
  localStorage.setItem(tokenStorageKey, token);
  setSessionCookie(true);
}

export function clearAccessToken() {
  if (typeof window === "undefined") return;
  localStorage.removeItem(tokenStorageKey);
  setSessionCookie(false);
}

export const apiClient = new ExpenseApiClient(
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080",
  getAccessToken
);
