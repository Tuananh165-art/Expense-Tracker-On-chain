import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: ["class"],
  content: [
    "./src/app/**/*.{ts,tsx}",
    "./src/components/**/*.{ts,tsx}",
    "./src/features/**/*.{ts,tsx}",
    "./src/lib/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        card: "hsl(var(--card))",
        "card-foreground": "hsl(var(--card-foreground))",
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        surface: {
          1: "hsl(var(--surface-1))",
          2: "hsl(var(--surface-2))",
          3: "hsl(var(--surface-3))",
        },
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        success: {
          DEFAULT: "hsl(var(--success))",
          foreground: "hsl(var(--success-foreground))",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      keyframes: {
        "fade-in-up": {
          "0%": { opacity: "0", transform: "translateY(12px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
      },
      animation: {
        "fade-in-up": "fade-in-up 420ms ease-out",
      },
      boxShadow: {
        glow: "0 0 0 1px hsl(var(--primary) / 0.3), 0 0 34px hsl(var(--primary) / 0.24)",
        elevated: "0 22px 55px -26px hsl(var(--background) / 0.9)",
        soft: "0 10px 26px -18px hsl(var(--background) / 0.78)",
        panel: "0 0 0 1px hsl(var(--border-strong) / 0.35), 0 18px 40px -26px hsl(var(--background) / 0.92)",
      },
      backgroundImage: {
        "chain-grid":
          "radial-gradient(circle at 14% 8%, hsl(var(--primary) / 0.18), transparent 38%), radial-gradient(circle at 86% 4%, hsl(var(--accent) / 0.2), transparent 34%), radial-gradient(circle at 50% 100%, hsl(var(--primary) / 0.1), transparent 40%), linear-gradient(145deg, hsl(var(--background)), hsl(var(--surface-1)))",
        "premium-sheen":
          "linear-gradient(135deg, hsl(var(--foreground) / 0.08), transparent 34%, hsl(var(--primary) / 0.12) 66%, transparent)",
        "aurora-panel":
          "radial-gradient(120% 120% at 0% 0%, hsl(var(--primary) / 0.14), transparent 45%), radial-gradient(100% 100% at 100% 0%, hsl(var(--accent) / 0.16), transparent 45%), linear-gradient(180deg, hsl(var(--surface-1)), hsl(var(--surface-2)))",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
