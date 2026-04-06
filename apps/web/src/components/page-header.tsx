import React from "react";

export function PageHeader({
  title,
  description,
  mode = "section",
}: {
  title: string;
  description: string;
  mode?: "hero" | "section";
}) {
  const isHero = mode === "hero";

  return (
    <header className={isHero ? "mb-8 space-y-2" : "mb-6 space-y-1.5"}>
      <h1
        className={
          isHero
            ? "bg-gradient-to-r from-foreground via-foreground to-foreground/75 bg-clip-text text-3xl font-bold tracking-tight text-transparent md:text-4xl"
            : "text-2xl font-bold tracking-tight"
        }
      >
        {title}
      </h1>
      <p className={isHero ? "max-w-2xl text-base text-muted-foreground" : "text-sm text-muted-foreground"}>
        {description}
      </p>
    </header>
  );
}
