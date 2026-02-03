import { PublicKey, TransactionInstruction, SystemProgram } from "@solana/web3.js";
import { PROGRAM_ID, DEPOSIT_DISCRIMINATOR, WITHDRAW_DISCRIMINATOR } from "./constants";
import { deriveVaultPDA } from "./pda";

/**
 * 创建 Deposit 指令
 * @param owner - Owner 公钥 (必须是签名者)
 * @param amount - 存款金额 (lamports)
 * @returns TransactionInstruction
 */
export function createDepositInstruction(owner: PublicKey, amount: bigint): TransactionInstruction {
  const [vaultPda] = deriveVaultPDA(owner);

  // 指令数据: [discriminator(1 byte)] + [amount(8 bytes, little-endian)]
  const data = Buffer.alloc(9);
  data.writeUInt8(DEPOSIT_DISCRIMINATOR, 0);
  data.writeBigUInt64LE(amount, 1);

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: vaultPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * 创建 Withdraw 指令
 * @param owner - Owner 公钥 (必须是签名者)
 * @returns TransactionInstruction
 */
export function createWithdrawInstruction(owner: PublicKey): TransactionInstruction {
  const [vaultPda] = deriveVaultPDA(owner);

  // 指令数据: [discriminator(1 byte)]
  const data = Buffer.alloc(1);
  data.writeUInt8(WITHDRAW_DISCRIMINATOR, 0);

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: vaultPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * 创建带自定义 vault 的 Deposit 指令 (用于测试错误情况)
 */
export function createDepositInstructionWithVault(
  owner: PublicKey,
  vault: PublicKey,
  amount: bigint
): TransactionInstruction {
  const data = Buffer.alloc(9);
  data.writeUInt8(DEPOSIT_DISCRIMINATOR, 0);
  data.writeBigUInt64LE(amount, 1);

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * 创建带自定义 vault 的 Withdraw 指令 (用于测试错误情况)
 */
export function createWithdrawInstructionWithVault(
  owner: PublicKey,
  vault: PublicKey
): TransactionInstruction {
  const data = Buffer.alloc(1);
  data.writeUInt8(WITHDRAW_DISCRIMINATOR, 0);

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * 创建带自定义数据的指令 (用于测试边界情况)
 */
export function createRawInstruction(owner: PublicKey, data: Buffer): TransactionInstruction {
  const [vaultPda] = deriveVaultPDA(owner);

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: owner, isSigner: true, isWritable: true },
      { pubkey: vaultPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}
