import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

describe("expense_program lifecycle", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ExpenseProgram as Program;
  const owner = provider.wallet;
  const stranger = anchor.web3.Keypair.generate();

  const categoryName = "Food";
  const expenseId = new anchor.BN(1);

  const [userProfilePda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("user_profile"), owner.publicKey.toBuffer()],
    program.programId
  );

  const [categoryPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("category"), owner.publicKey.toBuffer(), Buffer.from(categoryName)],
    program.programId
  );

  const [expensePda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("expense"), owner.publicKey.toBuffer(), expenseId.toArrayLike(Buffer, "le", 8)],
    program.programId
  );

  it("initializes user profile", async () => {
    await program.methods
      .initUserProfile()
      .accounts({
        owner: owner.publicKey,
        userProfile: userProfilePda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const profile = await program.account.userProfile.fetch(userProfilePda);
    if (!profile.owner.equals(owner.publicKey)) {
      throw new Error("owner mismatch in user profile");
    }
  });

  it("creates category", async () => {
    await program.methods
      .createCategory(categoryName)
      .accounts({
        owner: owner.publicKey,
        userProfile: userProfilePda,
        category: categoryPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const category = await program.account.category.fetch(categoryPda);
    if (category.name !== categoryName) {
      throw new Error("category name mismatch");
    }
    if (!category.owner.equals(owner.publicKey)) {
      throw new Error("category owner mismatch");
    }
  });

  it("creates expense", async () => {
    await program.methods
      .createExpense(expenseId, new anchor.BN(100_000), Array(32).fill(7))
      .accounts({
        owner: owner.publicKey,
        userProfile: userProfilePda,
        category: categoryPda,
        expense: expensePda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const expense = await program.account.expense.fetch(expensePda);
    if (!expense.owner.equals(owner.publicKey)) {
      throw new Error("expense owner mismatch");
    }
    if (expense.amount.toNumber() !== 100_000) {
      throw new Error("expense amount mismatch");
    }
    if (expense.status.pending == null) {
      throw new Error("expense status should be pending");
    }
  });

  it("updates expense status to approved", async () => {
    await program.methods
      .updateExpenseStatus({ approved: {} })
      .accounts({
        owner: owner.publicKey,
        expense: expensePda,
      })
      .rpc();

    const expense = await program.account.expense.fetch(expensePda);
    if (expense.status.approved == null) {
      throw new Error("expense status should be approved");
    }
  });

  it("fails creating expense when amount is zero", async () => {
    const [expenseZeroPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("expense"), owner.publicKey.toBuffer(), new anchor.BN(2).toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    let failed = false;
    try {
      await program.methods
        .createExpense(new anchor.BN(2), new anchor.BN(0), Array(32).fill(1))
        .accounts({
          owner: owner.publicKey,
          userProfile: userProfilePda,
          category: categoryPda,
          expense: expenseZeroPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    } catch {
      failed = true;
    }

    if (!failed) {
      throw new Error("expected createExpense with zero amount to fail");
    }
  });

  it("fails unauthorized status update", async () => {
    const airdropSig = await provider.connection.requestAirdrop(
      stranger.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSig, "confirmed");

    let failed = false;
    try {
      await program.methods
        .updateExpenseStatus({ rejected: {} })
        .accounts({
          owner: stranger.publicKey,
          expense: expensePda,
        })
        .signers([stranger])
        .rpc();
    } catch {
      failed = true;
    }

    if (!failed) {
      throw new Error("expected unauthorized status update to fail");
    }
  });

  it("fails when category name is empty", async () => {
    const [emptyCategoryPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("category"), owner.publicKey.toBuffer(), Buffer.from("")],
      program.programId
    );

    let failed = false;
    try {
      await program.methods
        .createCategory("")
        .accounts({
          owner: owner.publicKey,
          userProfile: userProfilePda,
          category: emptyCategoryPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    } catch {
      failed = true;
    }

    if (!failed) {
      throw new Error("expected empty category name to fail");
    }
  });
});
