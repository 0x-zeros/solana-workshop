import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BlueshiftAnchorVault } from "../target/types/blueshift_anchor_vault";
import { expect } from "chai";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";

describe("blueshift_anchor_vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.BlueshiftAnchorVault as Program<BlueshiftAnchorVault>;

  console.log("program.programId", program.programId);
  console.log("provider.wallet.publicKey", provider.wallet.publicKey);
  console.log("provider.connection.rpcEndpoint", provider.connection.rpcEndpoint);

  // 获取 vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), provider.wallet.publicKey.toBuffer()],
    program.programId
  );
  console.log("vaultPda", vaultPda);

  it("should deposit SOL to vault", async () => {
    // 存入 0.1 SOL
    const amount = 0.1 * LAMPORTS_PER_SOL;

    const tx = await program.methods
      .deposit(new anchor.BN(amount))
      // Anchor 新版本可以自动推断 PDA 账户（vault）和系统程序，无需手动指定
      // .accounts({
      //   signer: provider.wallet.publicKey,
      //   vault: vaultPda,
      //   systemProgram: anchor.web3.SystemProgram.programId,
      // })
      .rpc();

    console.log("Deposit transaction signature:", tx);

    // 验证 vault 余额
    const vaultBalance = await provider.connection.getBalance(vaultPda);
    expect(vaultBalance).to.equal(amount);
  });

  it("should fail to deposit to existing vault (VaultAlreadyExists)", async () => {
    // 尝试再次存款应该失败（使用不同金额避免重复交易检测）
    try {
      await program.methods
        .deposit(new anchor.BN(0.2 * LAMPORTS_PER_SOL))
        // Anchor 新版本可以自动推断 PDA 账户（vault）和系统程序，无需手动指定
        // .accounts({
        //   signer: provider.wallet.publicKey,
        //   vault: vaultPda,
        //   systemProgram: anchor.web3.SystemProgram.programId,
        // })
        .rpc();
      expect.fail("Should have thrown error");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("VaultAlreadyExists");
    }
  });

  it("should withdraw all SOL from vault", async () => {
    // 记录提取前余额
    const beforeBalance = await provider.connection.getBalance(provider.wallet.publicKey);

    const tx = await program.methods
      .withdraw()
      // Anchor 新版本可以自动推断 PDA 账户（vault）和系统程序，无需手动指定
      // .accounts({
      //   signer: provider.wallet.publicKey,
      //   vault: vaultPda,
      //   systemProgram: anchor.web3.SystemProgram.programId,
      // })
      .rpc();

    console.log("Withdraw transaction signature:", tx);

    // 验证 vault 余额为 0
    const vaultBalance = await provider.connection.getBalance(vaultPda);
    expect(vaultBalance).to.equal(0);

    // 验证 signer 收到了 vault 的 lamports（减去 gas 费用）
    const afterBalance = await provider.connection.getBalance(provider.wallet.publicKey);
    expect(afterBalance).to.be.greaterThan(beforeBalance);
  });

  it("should fail to withdraw from empty vault (InvalidAmount)", async () => {
    // 验证 vault 确实是空的
    const vaultBalance = await provider.connection.getBalance(vaultPda);
    expect(vaultBalance).to.equal(0);

    // 尝试从空 vault 提取应该失败
    // 使用 simulate 而不是 rpc 来避免重复交易检测问题
    try {
      await program.methods
        .withdraw()
        // Anchor 新版本可以自动推断 PDA 账户（vault）和系统程序，无需手动指定
        // .accounts({
        //   signer: provider.wallet.publicKey,
        //   vault: vaultPda,
        //   systemProgram: anchor.web3.SystemProgram.programId,
        // })
        .simulate();
      expect.fail("Should have thrown error");
    } catch (err: any) {
      // simulate 返回 SimulateError，需要从 logs 中验证错误
      if (err.simulationResponse?.logs) {
        const logs = err.simulationResponse.logs.join("\n");
        expect(logs).to.include("InvalidAmount");
      } else if (err.error?.errorCode?.code) {
        expect(err.error.errorCode.code).to.equal("InvalidAmount");
      } else {
        throw err;
      }
    }
  });

  it("should fail to deposit insufficient amount (InvalidAmount)", async () => {
    // 获取租金免除最低值，然后减去一些
    const rentExemptMinimum = await provider.connection.getMinimumBalanceForRentExemption(0);
    const tooSmallAmount = rentExemptMinimum - 1;

    try {
      await program.methods
        .deposit(new anchor.BN(tooSmallAmount))
        // Anchor 新版本可以自动推断 PDA 账户（vault）和系统程序，无需手动指定
        // .accounts({
        //   signer: provider.wallet.publicKey,
        //   vault: vaultPda,
        //   systemProgram: anchor.web3.SystemProgram.programId,
        // })
        .rpc();
      expect.fail("Should have thrown error");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("InvalidAmount");
    }
  });
});
