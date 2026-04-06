"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { clearAccessToken, getAccessToken } from "../lib/sdk";
import { cn } from "../lib/utils";
import { useMe } from "../features/auth/useWalletAuth";
import { Badge } from "./ui/badge";
import { Button, buttonVariants } from "./ui/button";

const items = [
  { href: "/dashboard", label: "Overview" },
  { href: "/dashboard/categories", label: "Categories" },
  { href: "/dashboard/expenses", label: "Expenses" },
  { href: "/dashboard/reports", label: "Reports" },
];

function isItemActive(pathname: string, href: string) {
  return href === "/dashboard" ? pathname === "/dashboard" : pathname === href || pathname.startsWith(`${href}/`);
}

function shortWallet(wallet?: string) {
  if (!wallet) return "";
  if (wallet.length <= 12) return wallet;
  return `${wallet.slice(0, 4)}...${wallet.slice(-4)}`;
}

function avatarText(role?: string, wallet?: string) {
  if (role) return role.slice(0, 1).toUpperCase();
  if (wallet) return wallet.slice(0, 1).toUpperCase();
  return "G";
}

export function AppNav() {
  const pathname = usePathname();
  const router = useRouter();
  const accountMenuRef = useRef<HTMLDivElement | null>(null);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [isAuthed, setIsAuthed] = useState(false);
  const [isAccountMenuOpen, setIsAccountMenuOpen] = useState(false);
  const [isMobileAccountOpen, setIsMobileAccountOpen] = useState(false);

  useEffect(() => {
    setIsAuthed(Boolean(getAccessToken()));
  }, []);

  const meQuery = useMe(isAuthed);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMobileOpen(false);
        setIsAccountMenuOpen(false);
        setIsMobileAccountOpen(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    if (!isAccountMenuOpen) return;
    const onClickOutside = (event: MouseEvent) => {
      if (!accountMenuRef.current?.contains(event.target as Node)) {
        setIsAccountMenuOpen(false);
      }
    };
    window.addEventListener("mousedown", onClickOutside);
    return () => window.removeEventListener("mousedown", onClickOutside);
  }, [isAccountMenuOpen]);

  const account = useMemo(() => {
    const wallet = meQuery.data?.wallet_address;
    const role = meQuery.data?.role;
    return {
      wallet,
      role,
      shortWallet: shortWallet(wallet),
      avatar: avatarText(role, wallet),
    };
  }, [meQuery.data?.wallet_address, meQuery.data?.role]);

  return (
    <>
      <nav className="sticky top-0 z-30 border-b border-[hsl(var(--border-strong))]/35 bg-surface-1/85 backdrop-blur">
        <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-2 px-4 py-3 md:px-8">
          <div className="flex items-center gap-2">
            <Link href="/" className="text-sm font-semibold tracking-wide text-foreground">
              Expense Tracker
            </Link>
            <Badge variant="muted" className="hidden md:inline-flex">
              Dashboard
            </Badge>
          </div>

          <div className="hidden items-center gap-2 md:flex">
            {items.map((item) => {
              const active = isItemActive(pathname, item.href);
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  aria-current={active ? "page" : undefined}
                  className={cn(
                    buttonVariants({ variant: active ? "default" : "ghost", size: "sm" }),
                    active && "border border-primary/40 shadow-glow font-semibold"
                  )}
                >
                  {item.label}
                </Link>
              );
            })}
          </div>

          <div ref={accountMenuRef} className="relative hidden items-center gap-2 md:flex">
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md border border-border/70 bg-card/70 px-2.5 py-1.5"
              onClick={() => {
                setIsMobileAccountOpen(false);
                setIsAccountMenuOpen((prev) => !prev);
              }}
              aria-expanded={isAccountMenuOpen}
              aria-haspopup="menu"
            >
              <span className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-primary/35 bg-primary/20 text-xs font-semibold text-foreground">
                {account.avatar}
              </span>
              <span className="text-xs text-muted-foreground">{isAuthed ? account.shortWallet || "Signed In" : "Guest"}</span>
            </button>

            <div
              className={cn(
                "absolute right-0 top-11 z-40 min-w-52 origin-top-right rounded-md border border-border/70 bg-surface-1 p-2 shadow-panel transition-all duration-300 ease-out",
                isAccountMenuOpen ? "visible translate-y-0 scale-100 opacity-100" : "invisible -translate-y-1 scale-95 opacity-0 pointer-events-none"
              )}
              role="menu"
              aria-hidden={!isAccountMenuOpen}
            >
              {isAuthed ? (
                <>
                  <Link
                    href="/dashboard"
                    className={cn(buttonVariants({ variant: "ghost", size: "sm" }), "w-full justify-start")}
                    onClick={() => setIsAccountMenuOpen(false)}
                  >
                    Dashboard
                  </Link>
                  <Button
                    variant="secondary"
                    size="sm"
                    className="mt-1 w-full justify-start"
                    onClick={() => {
                      clearAccessToken();
                      setIsAuthed(false);
                      setIsAccountMenuOpen(false);
                      router.replace("/sign-in");
                    }}
                  >
                    Logout
                  </Button>
                </>
              ) : (
                <Link
                  href="/sign-in"
                  className={cn(buttonVariants({ variant: "default", size: "sm" }), "w-full justify-start")}
                  onClick={() => setIsAccountMenuOpen(false)}
                >
                  Sign In With Wallet
                </Link>
              )}
            </div>
          </div>

          <button
            type="button"
            className={cn(buttonVariants({ variant: "secondary", size: "sm" }), "md:hidden")}
            aria-expanded={mobileOpen}
            aria-controls="mobile-nav-sheet"
            onClick={() => setMobileOpen(true)}
          >
            Menu
          </button>
        </div>
      </nav>

      {mobileOpen ? (
        <div className="fixed inset-0 z-40 md:hidden" role="dialog" aria-modal="true" id="mobile-nav-sheet">
          <button
            type="button"
            className="absolute inset-0 bg-black/55"
            aria-label="Close menu"
            onClick={() => {
              setIsMobileAccountOpen(false);
              setMobileOpen(false);
            }}
          />

          <aside className="absolute right-0 top-0 flex h-full w-72 animate-fade-in-up flex-col border-l border-[hsl(var(--border-strong))]/40 bg-surface-1 p-4 shadow-panel">
            <div className="mb-4 flex items-center justify-between">
              <span className="text-sm font-semibold">Navigation</span>
              <button
                type="button"
                className={cn(buttonVariants({ variant: "ghost", size: "sm" }))}
                onClick={() => {
                  setIsMobileAccountOpen(false);
                  setMobileOpen(false);
                }}
              >
                Close
              </button>
            </div>

            <div className="flex flex-col gap-2">
              {items.map((item) => {
                const active = isItemActive(pathname, item.href);
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      buttonVariants({ variant: active ? "default" : "secondary", size: "sm" }),
                      "justify-start",
                      active && "border border-primary/35 shadow-glow"
                    )}
                    onClick={() => {
                      setIsMobileAccountOpen(false);
                      setMobileOpen(false);
                    }}
                  >
                    {item.label}
                  </Link>
                );
              })}
            </div>

            <div className="mt-auto border-t border-border/70 pt-4">
              <button
                type="button"
                className="inline-flex w-full items-center justify-between rounded-md border border-border/70 bg-card/70 px-3 py-2"
                onClick={() => setIsMobileAccountOpen((prev) => !prev)}
                aria-expanded={isMobileAccountOpen}
                aria-controls="mobile-account-submenu"
              >
                <span className="inline-flex items-center gap-2">
                  <span className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-primary/35 bg-primary/20 text-xs font-semibold text-foreground">
                    {account.avatar}
                  </span>
                  <span className="text-sm">Account</span>
                </span>
                <span className="text-xs text-muted-foreground">{isAuthed ? account.shortWallet || "signed in" : "guest"}</span>
              </button>

              <div
                id="mobile-account-submenu"
                className={cn(
                  "mt-2 grid transition-all duration-300 ease-out",
                  isMobileAccountOpen ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0"
                )}
                aria-hidden={!isMobileAccountOpen}
              >
                <div className="min-h-0 overflow-hidden">
                  <div className="space-y-1 rounded-md border border-border/60 bg-surface-2/75 p-1">
                    {isAuthed ? (
                      <>
                        <Link
                          href="/dashboard"
                          className={cn(buttonVariants({ variant: "ghost", size: "sm" }), "w-full justify-start")}
                          onClick={() => {
                            setIsMobileAccountOpen(false);
                            setMobileOpen(false);
                          }}
                        >
                          Dashboard
                        </Link>
                        <Button
                          size="sm"
                          variant="secondary"
                          className="w-full justify-start"
                          onClick={() => {
                            clearAccessToken();
                            setIsAuthed(false);
                            setIsMobileAccountOpen(false);
                            setMobileOpen(false);
                            router.replace("/sign-in");
                          }}
                        >
                          Logout
                        </Button>
                      </>
                    ) : (
                      <Link
                        href="/sign-in"
                        className={cn(buttonVariants({ variant: "default", size: "sm" }), "w-full justify-start")}
                        onClick={() => {
                          setIsMobileAccountOpen(false);
                          setMobileOpen(false);
                        }}
                      >
                        Sign In With Wallet
                      </Link>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </aside>
        </div>
      ) : null}
    </>
  );
}
