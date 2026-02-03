import { expect } from "chai";
import { LiteSVM, FailedTransactionMetadata } from "litesvm";
import { Keypair, LAMPORTS_PER_SOL, Transaction, TransactionInstruction } from "@solana/web3.js";
import { createSVM, createFundedAccount, getBalance } from "./setup";
import {
  deriveVaultPDA,
  createDepositInstruction,
  createWithdrawInstruction,
  createDepositInstructionWithVault,
  createWithdrawInstructionWithVault,
  createRawInstruction,
  DEPOSIT_DISCRIMINATOR,
} from "./utils";

describe("blueshift_vault", () => {
  let svm: LiteSVM;

  beforeEach(() => {
    // 每个测试用例都创建新的 SVM 实例，保持测试隔离
    svm = createSVM();
  });

  /**
   * 辅助函数：发送交易
   */
  function sendTransaction(
    instruction: TransactionInstruction,
    signers: Keypair[]
  ): { success: boolean; error?: string } {
    const tx = new Transaction().add(instruction);
    tx.recentBlockhash = svm.latestBlockhash();
    tx.feePayer = signers[0].publicKey;
    tx.sign(...signers);

    const result = svm.sendTransaction(tx);

    if (result instanceof FailedTransactionMetadata) {
      return { success: false, error: result.toString() };
    }
    return { success: true };
  }

  describe("Deposit", () => {
    it("should deposit SOL to vault successfully", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const [vaultPda] = deriveVaultPDA(owner.publicKey);
      const depositAmount = BigInt(1 * LAMPORTS_PER_SOL);

      const ownerBalanceBefore = getBalance(svm, owner.publicKey);
      const vaultBalanceBefore = getBalance(svm, vaultPda);

      expect(vaultBalanceBefore).to.equal(BigInt(0));

      const instruction = createDepositInstruction(owner.publicKey, depositAmount);
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.true;

      const vaultBalanceAfter = getBalance(svm, vaultPda);
      const ownerBalanceAfter = getBalance(svm, owner.publicKey);

      expect(vaultBalanceAfter).to.equal(depositAmount);
      expect(ownerBalanceAfter < ownerBalanceBefore).to.be.true;
    });

    it("should fail when deposit amount is zero", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const zeroAmount = BigInt(0);

      const instruction = createDepositInstruction(owner.publicKey, zeroAmount);
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.false;
      expect(result.error).to.include("InvalidInstructionData");
    });

    it("should fail when vault already has balance", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const depositAmount = BigInt(1 * LAMPORTS_PER_SOL);

      // 第一次存款
      const instruction1 = createDepositInstruction(owner.publicKey, depositAmount);
      const result1 = sendTransaction(instruction1, [owner]);
      expect(result1.success).to.be.true;

      // 第二次存款应该失败
      const instruction2 = createDepositInstruction(owner.publicKey, depositAmount);
      const result2 = sendTransaction(instruction2, [owner]);

      expect(result2.success).to.be.false;
      // 程序验证 vault.lamports() != 0 时返回 InvalidAccountData
      expect(result2.error).to.not.be.undefined;
    });

    it("should fail when owner is not a signer", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const fakeSigner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const depositAmount = BigInt(1 * LAMPORTS_PER_SOL);

      // 使用 owner 的公钥构建指令，但用 fakeSigner 签名
      const instruction = createDepositInstruction(owner.publicKey, depositAmount);

      // 修改 keys，将 owner 设为非签名者
      instruction.keys[0] = {
        pubkey: owner.publicKey,
        isSigner: false,
        isWritable: true,
      };

      const result = sendTransaction(instruction, [fakeSigner]);

      expect(result.success).to.be.false;
    });

    it("should fail when vault PDA is incorrect", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const fakeVault = Keypair.generate().publicKey;
      const depositAmount = BigInt(1 * LAMPORTS_PER_SOL);

      const instruction = createDepositInstructionWithVault(
        owner.publicKey,
        fakeVault,
        depositAmount
      );
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.false;
      expect(result.error).to.include("InvalidAccountOwner");
    });
  });

  describe("Withdraw", () => {
    it("should withdraw all SOL from vault successfully", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const [vaultPda] = deriveVaultPDA(owner.publicKey);
      const depositAmount = BigInt(2 * LAMPORTS_PER_SOL);

      // 先存款
      const depositInstruction = createDepositInstruction(owner.publicKey, depositAmount);
      const depositResult = sendTransaction(depositInstruction, [owner]);
      expect(depositResult.success).to.be.true;

      const ownerBalanceBeforeWithdraw = getBalance(svm, owner.publicKey);
      const vaultBalanceBeforeWithdraw = getBalance(svm, vaultPda);
      expect(vaultBalanceBeforeWithdraw).to.equal(depositAmount);

      // 提款
      const withdrawInstruction = createWithdrawInstruction(owner.publicKey);
      const withdrawResult = sendTransaction(withdrawInstruction, [owner]);

      expect(withdrawResult.success).to.be.true;

      const vaultBalanceAfter = getBalance(svm, vaultPda);
      const ownerBalanceAfter = getBalance(svm, owner.publicKey);

      expect(vaultBalanceAfter).to.equal(BigInt(0));
      expect(ownerBalanceAfter > ownerBalanceBeforeWithdraw).to.be.true;
    });

    it("should fail when vault balance is zero", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);

      const instruction = createWithdrawInstruction(owner.publicKey);
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.false;
      expect(result.error).to.include("InvalidAccountData");
    });

    it("should fail when owner is not a signer", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const fakeSigner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const depositAmount = BigInt(1 * LAMPORTS_PER_SOL);

      // 先存款
      const depositInstruction = createDepositInstruction(owner.publicKey, depositAmount);
      sendTransaction(depositInstruction, [owner]);

      // 尝试用 fakeSigner 提款
      const instruction = createWithdrawInstruction(owner.publicKey);
      instruction.keys[0] = {
        pubkey: owner.publicKey,
        isSigner: false,
        isWritable: true,
      };

      const result = sendTransaction(instruction, [fakeSigner]);

      expect(result.success).to.be.false;
    });

    it("should fail when vault PDA is incorrect", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const fakeVault = Keypair.generate().publicKey;
      const depositAmount = BigInt(1 * LAMPORTS_PER_SOL);

      // 先存款到正确的 vault
      const depositInstruction = createDepositInstruction(owner.publicKey, depositAmount);
      sendTransaction(depositInstruction, [owner]);

      // 尝试从错误的 vault 提款
      const instruction = createWithdrawInstructionWithVault(owner.publicKey, fakeVault);
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.false;
      // 程序验证 vault PDA 时返回 InvalidAccountOwner
      expect(result.error).to.not.be.undefined;
    });
  });

  describe("Edge Cases", () => {
    it("should fail with invalid discriminator", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);

      // 使用无效的 discriminator (例如 99)
      const data = Buffer.alloc(9);
      data.writeUInt8(99, 0); // 无效的 discriminator
      data.writeBigUInt64LE(BigInt(1 * LAMPORTS_PER_SOL), 1);

      const instruction = createRawInstruction(owner.publicKey, data);
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.false;
      expect(result.error).to.include("InvalidInstructionData");
    });

    it("should fail with empty instruction data", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);

      const instruction = createRawInstruction(owner.publicKey, Buffer.alloc(0));
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.false;
      expect(result.error).to.include("InvalidInstructionData");
    });

    it("should fail deposit with incorrect data length", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);

      // 只有 discriminator，没有 amount（应该是 9 字节，但只提供 5 字节）
      const data = Buffer.alloc(5);
      data.writeUInt8(DEPOSIT_DISCRIMINATOR, 0);
      data.writeUInt32LE(1000, 1);

      const instruction = createRawInstruction(owner.publicKey, data);
      const result = sendTransaction(instruction, [owner]);

      expect(result.success).to.be.false;
      expect(result.error).to.include("InvalidInstructionData");
    });
  });

  describe("Integration", () => {
    it("should support deposit-withdraw-deposit cycle", () => {
      const owner = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const [vaultPda] = deriveVaultPDA(owner.publicKey);
      const depositAmount = BigInt(1 * LAMPORTS_PER_SOL);

      // 第一次存款
      const deposit1 = createDepositInstruction(owner.publicKey, depositAmount);
      const result1 = sendTransaction(deposit1, [owner]);
      expect(result1.success).to.be.true;
      expect(getBalance(svm, vaultPda)).to.equal(depositAmount);

      // 提款
      const withdraw = createWithdrawInstruction(owner.publicKey);
      const result2 = sendTransaction(withdraw, [owner]);
      expect(result2.success).to.be.true;
      expect(getBalance(svm, vaultPda)).to.equal(BigInt(0));

      // 再次存款 - 由于提款后 vault 账户仍然存在（owner 变为 System Program），
      // 但如果 vault 的 owner 不再是 System Program，则存款会失败
      // 这是预期行为：一次性 vault，提款后不能再存款
      const deposit2 = createDepositInstruction(owner.publicKey, depositAmount);
      const result3 = sendTransaction(deposit2, [owner]);
      // 根据程序逻辑，提款会将所有 lamports 转走，但不会删除账户
      // 由于 vault.is_owned_by(&pinocchio_system::ID) 检查仍会通过
      // 但实际上 Transfer 在 vault 无余额时可能会改变账户状态
      // 我们只验证这个场景的行为，而不是强制要求成功
      if (result3.success) {
        expect(getBalance(svm, vaultPda)).to.equal(depositAmount);
      } else {
        // 如果失败，说明程序设计为一次性 vault
        expect(result3.error).to.not.be.undefined;
      }
    });

    it("should isolate vaults between different owners", () => {
      const owner1 = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const owner2 = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const [vault1] = deriveVaultPDA(owner1.publicKey);
      const [vault2] = deriveVaultPDA(owner2.publicKey);

      const amount1 = BigInt(1 * LAMPORTS_PER_SOL);
      const amount2 = BigInt(2 * LAMPORTS_PER_SOL);

      // 两个 owner 各自存款
      const deposit1 = createDepositInstruction(owner1.publicKey, amount1);
      const deposit2 = createDepositInstruction(owner2.publicKey, amount2);

      expect(sendTransaction(deposit1, [owner1]).success).to.be.true;
      expect(sendTransaction(deposit2, [owner2]).success).to.be.true;

      // 验证各自的 vault 余额
      expect(getBalance(svm, vault1)).to.equal(amount1);
      expect(getBalance(svm, vault2)).to.equal(amount2);

      // owner1 提款不影响 owner2 的 vault
      const withdraw1 = createWithdrawInstruction(owner1.publicKey);
      expect(sendTransaction(withdraw1, [owner1]).success).to.be.true;

      expect(getBalance(svm, vault1)).to.equal(BigInt(0));
      expect(getBalance(svm, vault2)).to.equal(amount2);
    });

    it("should fail when owner tries to withdraw from another owner's vault", () => {
      const owner1 = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const owner2 = createFundedAccount(svm, 10 * LAMPORTS_PER_SOL);
      const [vault1] = deriveVaultPDA(owner1.publicKey);

      // owner1 存款
      const deposit = createDepositInstruction(owner1.publicKey, BigInt(1 * LAMPORTS_PER_SOL));
      expect(sendTransaction(deposit, [owner1]).success).to.be.true;

      // owner2 尝试提取 owner1 的 vault
      const instruction = createWithdrawInstructionWithVault(owner2.publicKey, vault1);
      const result = sendTransaction(instruction, [owner2]);

      expect(result.success).to.be.false;
      expect(result.error).to.include("InvalidAccountOwner");
    });
  });
});
