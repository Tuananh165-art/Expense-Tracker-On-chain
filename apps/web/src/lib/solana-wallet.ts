"use client";

import bs58 from "bs58";
import { Connection, PublicKey, SystemProgram, Transaction, TransactionInstruction } from "@solana/web3.js";

type SignResult = Uint8Array | { signature: Uint8Array };
type SendTxResult = string | { signature: string };

const DISC_INIT_USER_PROFILE = Buffer.from([148, 35, 126, 247, 28, 169, 135, 175]);
const DISC_CREATE_CATEGORY = Buffer.from([220, 242, 238, 47, 228, 219, 223, 230]);
const DISC_CREATE_EXPENSE = Buffer.from([63, 19, 41, 230, 40, 213, 132, 147]);
const DISC_UPDATE_EXPENSE_STATUS = Buffer.from([194, 57, 187, 247, 173, 117, 239, 124]);

export interface SolanaProvider {
  isPhantom?: boolean;
  publicKey?: { toBase58: () => string };
  connect: () => Promise<{ publicKey: { toBase58: () => string } }>;
  signMessage?: (message: Uint8Array, encoding?: "utf8") => Promise<SignResult>;
  signAndSendTransaction?: (tx: Transaction) => Promise<SendTxResult>;
  signTransaction?: (tx: Transaction) => Promise<Transaction>;
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

export function getHybridConfig() {
  return {
    enabled: process.env.NEXT_PUBLIC_HYBRID_ONCHAIN_ENABLED === "true",
    rpcUrl: process.env.NEXT_PUBLIC_SOLANA_RPC_URL ?? "http://127.0.0.1:8899",
    programId: process.env.NEXT_PUBLIC_PROGRAM_ID ?? "rzMxNuut6R34aFgt8NY9hj3SoRB37iszrsSqZR2DSnB",
    commitment: (process.env.NEXT_PUBLIC_SOLANA_COMMITMENT as "processed" | "confirmed" | "finalized") ??
      "confirmed",
  };
}

export async function connectSolanaWallet(provider: SolanaProvider): Promise<string> {
  const result = await provider.connect();
  return result.publicKey.toBase58();
}

function concatBytes(...parts: Uint8Array[]): Buffer {
  return Buffer.concat(parts.map((p) => Buffer.from(p)));
}

function u64Le(value: number): Buffer {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(value));
  return out;
}

function stringArg(value: string): Buffer {
  const raw = Buffer.from(value, "utf8");
  const len = Buffer.alloc(4);
  len.writeUInt32LE(raw.length, 0);
  return Buffer.concat([len, raw]);
}

async function sendTransactionWithProvider(
  provider: SolanaProvider,
  connection: Connection,
  tx: Transaction
): Promise<string> {
  if (provider.signAndSendTransaction) {
    const res = await provider.signAndSendTransaction(tx);
    return typeof res === "string" ? res : res.signature;
  }

  if (!provider.signTransaction) {
    throw new Error("Wallet does not support transaction signing");
  }

  const signed = await provider.signTransaction(tx);
  return connection.sendRawTransaction(signed.serialize());
}

async function sendInstruction(
  provider: SolanaProvider,
  owner: PublicKey,
  ix: TransactionInstruction,
  commitment: "processed" | "confirmed" | "finalized"
): Promise<string> {
  const { rpcUrl } = getHybridConfig();
  const connection = new Connection(rpcUrl, "confirmed");
  const tx = new Transaction().add(ix);
  tx.feePayer = owner;
  tx.recentBlockhash = (await connection.getLatestBlockhash("confirmed")).blockhash;

  const txHash = await sendTransactionWithProvider(provider, connection, tx);
  await connection.confirmTransaction(txHash, commitment);
  return txHash;
}

async function ensureUserProfile(provider: SolanaProvider, owner: PublicKey): Promise<void> {
  const { rpcUrl, programId, commitment } = getHybridConfig();
  const connection = new Connection(rpcUrl, "confirmed");
  const programPubkey = new PublicKey(programId);

  const [userProfilePda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode("user_profile"), owner.toBytes()],
    programPubkey
  );

  const existing = await connection.getAccountInfo(userProfilePda, commitment);
  if (existing) return;

  const ix = new TransactionInstruction({
    programId: programPubkey,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: userProfilePda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: DISC_INIT_USER_PROFILE,
  });

  await sendInstruction(provider, owner, ix, commitment);
}

export function deriveCategoryPda(ownerWallet: string, categoryName: string, programId: string): string {
  const [pda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode("category"), new PublicKey(ownerWallet).toBytes(), new TextEncoder().encode(categoryName)],
    new PublicKey(programId)
  );
  return pda.toBase58();
}

export function deriveExpensePda(ownerWallet: string, expenseIdOnchain: number, programId: string): string {
  const [pda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode("expense"), new PublicKey(ownerWallet).toBytes(), u64Le(expenseIdOnchain)],
    new PublicKey(programId)
  );
  return pda.toBase58();
}

export function makeExpenseIdOnchain(): number {
  return Number(Date.now() % 1_000_000_000);
}

export async function sendCreateCategoryTx(
  provider: SolanaProvider,
  ownerWallet: string,
  categoryName: string
): Promise<{ txHash: string; categoryPda: string }> {
  const { programId, commitment } = getHybridConfig();
  const owner = new PublicKey(ownerWallet);
  const programPubkey = new PublicKey(programId);

  await ensureUserProfile(provider, owner);

  const [userProfilePda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode("user_profile"), owner.toBytes()],
    programPubkey
  );
  const [categoryPda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode("category"), owner.toBytes(), new TextEncoder().encode(categoryName)],
    programPubkey
  );

  const ix = new TransactionInstruction({
    programId: programPubkey,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: userProfilePda, isSigner: false, isWritable: false },
      { pubkey: categoryPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBytes(DISC_CREATE_CATEGORY, stringArg(categoryName)),
  });

  const txHash = await sendInstruction(provider, owner, ix, commitment);
  return { txHash, categoryPda: categoryPda.toBase58() };
}

export async function sendCreateExpenseTx(
  provider: SolanaProvider,
  ownerWallet: string,
  categoryPda: string,
  expenseIdOnchain: number,
  amountMinor: number
): Promise<{ txHash: string; expensePda: string }> {
  const { programId, commitment } = getHybridConfig();
  const owner = new PublicKey(ownerWallet);
  const programPubkey = new PublicKey(programId);

  await ensureUserProfile(provider, owner);

  const [userProfilePda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode("user_profile"), owner.toBytes()],
    programPubkey
  );
  const [expensePda] = PublicKey.findProgramAddressSync(
    [new TextEncoder().encode("expense"), owner.toBytes(), u64Le(expenseIdOnchain)],
    programPubkey
  );

  const noteHash = new Uint8Array(32);
  noteHash.fill(7);

  const ix = new TransactionInstruction({
    programId: programPubkey,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: userProfilePda, isSigner: false, isWritable: false },
      { pubkey: new PublicKey(categoryPda), isSigner: false, isWritable: false },
      { pubkey: expensePda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBytes(DISC_CREATE_EXPENSE, u64Le(expenseIdOnchain), u64Le(amountMinor), noteHash),
  });

  const txHash = await sendInstruction(provider, owner, ix, commitment);
  return { txHash, expensePda: expensePda.toBase58() };
}

export async function sendUpdateExpenseStatusTx(
  provider: SolanaProvider,
  ownerWallet: string,
  expensePda: string,
  toStatus: "approved" | "rejected"
): Promise<string> {
  const { programId, commitment } = getHybridConfig();
  const owner = new PublicKey(ownerWallet);
  const statusByte = toStatus === "approved" ? 1 : 2;

  const ix = new TransactionInstruction({
    programId: new PublicKey(programId),
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: new PublicKey(expensePda), isSigner: false, isWritable: true },
    ],
    data: concatBytes(DISC_UPDATE_EXPENSE_STATUS, new Uint8Array([statusByte])),
  });

  return sendInstruction(provider, owner, ix, commitment);
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
