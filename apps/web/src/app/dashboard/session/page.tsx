"use client";

import React, { useEffect } from "react";
import { useRouter } from "next/navigation";

export default function SessionRedirectPage() {
  const router = useRouter();

  useEffect(() => {
    router.replace("/sign-in");
  }, [router]);

  return null;
}
