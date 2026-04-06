import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  transpilePackages: ["@expense/sdk", "@expense/shared"],
};

export default nextConfig;
