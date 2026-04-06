"use client";

import bs58 from "bs58";

type SignResult = Uint8Array | { signature: Uint8Array };

export interface SolanaProvider {
  isPhantom?: boolean;
  publicKey?: { toBase58: () => string };
  connect: () => Promise<{ publicKey: { toBase58: () => string } }>;
  signMessage?: (message: Uint8Array, encoding?: "utf8") => Promise<SignResult>;
}

interface WindowWithSolana extends Window {
  solana?: SolanaProvider;
  phantom?: { solana?: SolanaProvider };
  solflare?: { isSolflare?: boolean } & SolanaProvider;
}

export function getSolanaProvider(): SolanaProvider | null {
  if (typeof window === "undefined") return null;
  const maybeWindow = window as WindowWithSolana;

  if (maybeWindow.solana?.isPhantom) return maybeWindow.solana;
  if (maybeWindow.phantom?.solana) return maybeWindow.phantom.solana;
  if (maybeWindow.solana) return maybeWindow.solana;
  if (maybeWindow.solflare) return maybeWindow.solflare;

  return null;
}

export async function connectSolanaWallet(provider: SolanaProvider): Promise<string> {
  const result = await provider.connect();
  return result.publicKey.toBase58();
}

export async function signChallengeMessage(provider: SolanaProvider, message: string): Promise<string> {
  if (!provider.signMessage) {
    throw new Error("Wallet does not support signMessage");
  }

  const encoded = new TextEncoder().encode(message);

  let signed: SignResult;
  try {
    signed = await provider.signMessage(encoded, "utf8");
  } catch {
    signed = await provider.signMessage(encoded);
  }

  const signature = signed instanceof Uint8Array ? signed : signed.signature;
  return bs58.encode(signature);
}
