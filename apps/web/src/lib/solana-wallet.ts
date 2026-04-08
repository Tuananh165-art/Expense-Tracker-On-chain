"use client";

import bs58 from "bs58";
import {
  Connection,
  PublicKey,
  SendTransactionError,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

type SignResult = Uint8Array | { signature: Uint8Array };
type SendTxResult = string | { signature: string };

const DISC_INIT_PROGRAM_CONFIG = Buffer.from([185, 54, 237, 229, 219, 179, 109, 20]);
const DISC_INIT_USER_PROFILE = Buffer.from([148, 35, 126, 247, 28, 169, 135, 175]);
const DISC_CREATE_CATEGORY = Buffer.from([220, 242, 238, 47, 228, 219, 223, 230]);
const DISC_CREATE_EXPENSE = Buffer.from([63, 19, 41, 230, 40, 213, 132, 147]);
const DISC_UPDATE_EXPENSE_STATUS = Buffer.from([194, 57, 187, 247, 173, 117, 239, 124]);
const DISC_ACCOUNT_EXPENSE = Buffer.from([49, 167, 206, 160, 209, 254, 24, 100]);

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

export function isAirdropAllowedCluster(rpcUrl: string): boolean {
  const value = rpcUrl.toLowerCase();
  if (value.includes("mainnet")) return false;
  return value.includes("127.0.0.1") || value.includes("localhost") || value.includes("devnet");
}

export async function getWalletBalanceLamports(ownerWallet: string): Promise<{ lamports: number; rpcUrl: string }> {
  const { rpcUrl } = getHybridConfig();
  const connection = new Connection(rpcUrl, "confirmed");
  const lamports = await connection.getBalance(new PublicKey(ownerWallet), "confirmed");
  return { lamports, rpcUrl };
}

export async function requestSolAirdrop(
  ownerWallet: string,
  lamports = 1_000_000_000
): Promise<{ signature: string; lamportsAfter: number; rpcUrl: string }> {
  const { rpcUrl, commitment } = getHybridConfig();
  if (!isAirdropAllowedCluster(rpcUrl)) {
    throw new Error(`Airdrop is not allowed on ${rpcUrl}. Switch to localhost/devnet or fund wallet manually.`);
  }

  const connection = new Connection(rpcUrl, "confirmed");
  const owner = new PublicKey(ownerWallet);
  const signature = await connection.requestAirdrop(owner, lamports);
  await connection.confirmTransaction(signature, commitment);
  const lamportsAfter = await connection.getBalance(owner, "confirmed");

  return { signature, lamportsAfter, rpcUrl };
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

function pubkeyArg(pubkey: PublicKey): Buffer {
  return Buffer.from(pubkey.toBytes());
}

function isUnexpectedWalletError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  return error.message.toLowerCase().includes("unexpected error");
}

function isCategoryAlreadyExistsError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  const message = error.message.toLowerCase();
  return message.includes("already in use") || message.includes("category already exists on-chain");
}

function isUninitializedExpenseError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  const message = error.message.toLowerCase();
  return message.includes("accountnotinitialized") && message.includes("account: expense");
}

function isUserRejectedWalletError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  const message = error.message.toLowerCase();
  return message.includes("user rejected") || message.includes("rejected the request") || message.includes("cancelled");
}

async function sendTransactionWithProvider(
  provider: SolanaProvider,
  connection: Connection,
  tx: Transaction
): Promise<string> {
  if (provider.signAndSendTransaction) {
    try {
      const res = await provider.signAndSendTransaction(tx);
      return typeof res === "string" ? res : res.signature;
    } catch (error) {
      if (isUserRejectedWalletError(error)) {
        throw new Error("Wallet request was rejected by user.");
      }
      if (!provider.signTransaction) {
        throw error;
      }
      if (!isUnexpectedWalletError(error)) {
        throw error;
      }
    }
  }

  if (!provider.signTransaction) {
    throw new Error("Wallet does not support transaction signing");
  }

  tx.recentBlockhash = (await connection.getLatestBlockhash("confirmed")).blockhash;
  let signed: Transaction;
  try {
    signed = await provider.signTransaction(tx);
  } catch (error) {
    if (isUserRejectedWalletError(error)) {
      throw new Error("Wallet request was rejected by user.");
    }
    throw error;
  }

  return connection.sendRawTransaction(signed.serialize(), {
    preflightCommitment: "confirmed",
    maxRetries: 3,
  });
}

async function sendInstruction(
  provider: SolanaProvider,
  owner: PublicKey,
  ix: TransactionInstruction,
  commitment: "processed" | "confirmed" | "finalized"
): Promise<string> {
  const { rpcUrl, programId } = getHybridConfig();
  const connection = new Connection(rpcUrl, "confirmed");

  const programAccount = await connection.getAccountInfo(new PublicKey(programId), "confirmed");
  if (!programAccount?.executable) {
    throw new Error(
      `Program ${programId} is not deployed on ${rpcUrl}. Run localnet deploy and ensure NEXT_PUBLIC_PROGRAM_ID matches.`
    );
  }

  const ownerBalance = await connection.getBalance(owner, "confirmed");
  if (ownerBalance <= 0) {
    throw new Error(
      `Wallet has 0 SOL on ${rpcUrl}. Fund this wallet on the same cluster before sending transactions.`
    );
  }

  for (let attempt = 1; attempt <= 2; attempt += 1) {
    const tx = new Transaction().add(ix);
    tx.feePayer = owner;
    tx.recentBlockhash = (await connection.getLatestBlockhash("confirmed")).blockhash;

    try {
      const txHash = await sendTransactionWithProvider(provider, connection, tx);
      await connection.confirmTransaction(txHash, commitment);
      return txHash;
    } catch (error) {
      if (error instanceof SendTransactionError) {
        const message = error.message;
        if (message.toLowerCase().includes("blockhash not found") && attempt < 2) {
          continue;
        }

        let details = "";
        try {
          const logs = await error.getLogs(connection);
          if (logs && logs.length > 0) {
            details = ` Logs: ${logs.join(" | ")}`;
          }
        } catch {
          // no-op
        }
        const simulationError = new Error(`Transaction simulation failed. ${message}${details}`.trim());
        if (isCategoryAlreadyExistsError(simulationError)) {
          throw new Error("Category already exists on-chain");
        }
        if (isUninitializedExpenseError(simulationError)) {
          throw new Error(
            "On-chain expense account is not initialized on current RPC. Localnet/program data is out of sync. Re-create expense before approve/reject."
          );
        }
        throw simulationError;
      }
      throw error;
    }
  }

  throw new Error("Transaction failed after retry.");
}

async function ensureProgramConfig(provider: SolanaProvider, signer: PublicKey): Promise<void> {
  const { rpcUrl, programId, commitment } = getHybridConfig();
  const connection = new Connection(rpcUrl, "confirmed");
  const programPubkey = new PublicKey(programId);
  const programConfigPda = new PublicKey(deriveProgramConfigPda(programId));

  const existing = await connection.getAccountInfo(programConfigPda, commitment);
  if (existing) return;

  const ix = new TransactionInstruction({
    programId: programPubkey,
    keys: [
      { pubkey: signer, isSigner: true, isWritable: true },
      { pubkey: programConfigPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: concatBytes(DISC_INIT_PROGRAM_CONFIG, pubkeyArg(signer)),
  });

  await sendInstruction(provider, signer, ix, commitment);
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

export function deriveProgramConfigPda(programId: string): string {
  const [pda] = PublicKey.findProgramAddressSync([new TextEncoder().encode("program_config")], new PublicKey(programId));
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
  const { rpcUrl, programId, commitment } = getHybridConfig();
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

  const connection = new Connection(rpcUrl, "confirmed");
  const categoryAccount = await connection.getAccountInfo(categoryPda, commitment);
  if (categoryAccount) {
    throw new Error("Category already exists on-chain");
  }

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
  signerWallet: string,
  expensePda: string,
  toStatus: "approved" | "rejected"
): Promise<string> {
  const { rpcUrl, programId, commitment } = getHybridConfig();
  const signer = new PublicKey(signerWallet);
  const programConfigPda = new PublicKey(deriveProgramConfigPda(programId));
  const statusByte = toStatus === "approved" ? 1 : 2;

  await ensureProgramConfig(provider, signer);

  const connection = new Connection(rpcUrl, "confirmed");
  const expenseAccount = await connection.getAccountInfo(new PublicKey(expensePda), commitment);
  if (!expenseAccount) {
    throw new Error(
      "On-chain expense account was not found on current RPC. This usually means localnet was reset and DB metadata is stale. Re-create expense before approve/reject."
    );
  }

  if (!expenseAccount.owner.equals(new PublicKey(programId))) {
    throw new Error(
      "On-chain expense PDA exists but is not owned by current program. Localnet/program metadata is out of sync. Re-create expense after redeploy."
    );
  }

  if (expenseAccount.data.length < 8 || !expenseAccount.data.subarray(0, 8).equals(DISC_ACCOUNT_EXPENSE)) {
    throw new Error(
      "On-chain expense account is not initialized for current program schema. Localnet data is stale. Re-create expense before approve/reject."
    );
  }

  const ix = new TransactionInstruction({
    programId: new PublicKey(programId),
    keys: [
      { pubkey: signer, isSigner: true, isWritable: true },
      { pubkey: programConfigPda, isSigner: false, isWritable: false },
      { pubkey: new PublicKey(expensePda), isSigner: false, isWritable: true },
    ],
    data: concatBytes(DISC_UPDATE_EXPENSE_STATUS, new Uint8Array([statusByte])),
  });

  return sendInstruction(provider, signer, ix, commitment);
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
