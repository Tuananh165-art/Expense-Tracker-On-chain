const anchor = require("@coral-xyz/anchor");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const { PublicKey, Keypair, Connection, Transaction, TransactionInstruction, SystemProgram } =
  anchor.web3;

function disc(name) {
  return crypto.createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

function u64le(n) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n));
  return b;
}

function stringArg(s) {
  const raw = Buffer.from(s, "utf8");
  const len = Buffer.alloc(4);
  len.writeUInt32LE(raw.length);
  return Buffer.concat([len, raw]);
}

async function sendAndConfirm(connection, payer, ix) {
  const tx = new Transaction().add(ix);
  tx.feePayer = payer.publicKey;
  return anchor.web3.sendAndConfirmTransaction(connection, tx, [payer], {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
    maxRetries: 5,
  });
}

async function main() {
  const rpc = process.env.SOLANA_RPC_URL || "http://127.0.0.1:8899";
  const programId = new PublicKey(
    process.env.SOLANA_PROGRAM_ID || "rzMxNuut6R34aFgt8NY9hj3SoRB37iszrsSqZR2DSnB"
  );

  const keypairPath = process.env.SOLANA_KEYPAIR_PATH
    ? path.resolve(process.env.SOLANA_KEYPAIR_PATH)
    : path.resolve(process.env.HOME, ".config/solana/id.json");
  const secret = Uint8Array.from(JSON.parse(fs.readFileSync(keypairPath, "utf8")));
  const payer = Keypair.fromSecretKey(secret);
  const owner = payer.publicKey;
  const connection = new Connection(rpc, "finalized");

  const categoryName = `food_e2e_${Date.now()}`;
  const expenseIdOnchain = Number(Date.now() % 1000000);
  const amountMinor = 123456;
  const noteHash = Buffer.alloc(32, 7);

  const [userProfilePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("user_profile"), owner.toBuffer()],
    programId
  );
  const [categoryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("category"), owner.toBuffer(), Buffer.from(categoryName)],
    programId
  );
  const [expensePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("expense"), owner.toBuffer(), u64le(expenseIdOnchain)],
    programId
  );

  const existingProfile = await connection.getAccountInfo(userProfilePda, "finalized");
  if (!existingProfile) {
    const initIx = new TransactionInstruction({
      programId,
      keys: [
        { pubkey: owner, isSigner: true, isWritable: true },
        { pubkey: userProfilePda, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: disc("init_user_profile"),
    });
    await sendAndConfirm(connection, payer, initIx);
  }

  const categoryIx = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: userProfilePda, isSigner: false, isWritable: false },
      { pubkey: categoryPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([disc("create_category"), stringArg(categoryName)]),
  });
  const categoryTx = await sendAndConfirm(connection, payer, categoryIx);

  const createIx = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: userProfilePda, isSigner: false, isWritable: false },
      { pubkey: categoryPda, isSigner: false, isWritable: false },
      { pubkey: expensePda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([disc("create_expense"), u64le(expenseIdOnchain), u64le(amountMinor), noteHash]),
  });
  const expenseCreateTx = await sendAndConfirm(connection, payer, createIx);

  const statusIx = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: expensePda, isSigner: false, isWritable: true },
    ],
    data: Buffer.concat([disc("update_expense_status"), Buffer.from([1])]),
  });
  const expenseStatusTx = await sendAndConfirm(connection, payer, statusIx);

  console.log(`export CATEGORY_NAME='${categoryName}'`);
  console.log(`export CATEGORY_PDA='${categoryPda.toBase58()}'`);
  console.log(`export EXPENSE_ID_ONCHAIN='${expenseIdOnchain}'`);
  console.log(`export AMOUNT_MINOR='${amountMinor}'`);
  console.log(`export CATEGORY_TX_HASH='${categoryTx}'`);
  console.log(`export EXPENSE_CREATE_TX_HASH='${expenseCreateTx}'`);
  console.log(`export EXPENSE_STATUS_TX_HASH='${expenseStatusTx}'`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
