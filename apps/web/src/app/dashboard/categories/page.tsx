"use client";

import Link from "next/link";
import React, { useEffect, useRef, useState } from "react";
import { getAccessToken } from "../../../lib/sdk";
import { useCategories, useCreateCategory } from "../../../features/categories/useCategories";
import { Button, buttonVariants } from "../../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../../components/ui/card";
import { Input } from "../../../components/ui/input";
import { PageHeader } from "../../../components/page-header";
import { cn } from "../../../lib/utils";

export default function CategoriesPage() {
  const [isAuthed, setIsAuthed] = useState(false);
  const [categoryName, setCategoryName] = useState("");
  const categoryInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    setIsAuthed(Boolean(getAccessToken()));
  }, []);

  const categoriesQuery = useCategories(isAuthed);
  const createCategoryMutation = useCreateCategory();

  return (
    <div className="space-y-8">
      <PageHeader title="Categories" description="Tạo category mới và quản lý danh mục chi tiêu." />

      <div className="surface-toolbar">
        <Button size="sm" onClick={() => categoryInputRef.current?.focus()}>
          New Category
        </Button>
        <Link href="/dashboard/expenses" className={cn(buttonVariants({ variant: "secondary", size: "sm" }))}>
          Go to Expenses
        </Link>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Create category</CardTitle>
          <CardDescription>Tạo category để phân loại expense.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex gap-2">
            <Input
              ref={categoryInputRef}
              placeholder="Category name"
              value={categoryName}
              onChange={(e) => setCategoryName(e.target.value)}
            />
            <Button
              variant="secondary"
              onClick={async () => {
                await createCategoryMutation.mutateAsync(categoryName);
                setCategoryName("");
              }}
              disabled={!isAuthed || !categoryName}
            >
              Create
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Category list</CardTitle>
          <CardDescription>Danh sách category hiện tại.</CardDescription>
        </CardHeader>
        <CardContent>
          <ul className="space-y-1 text-sm text-muted-foreground">
            {(categoriesQuery.data ?? []).map((c) => (
              <li key={c.id} className="surface-list-item">
                <span className="font-medium text-foreground">{c.name}</span> • {c.id}
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
