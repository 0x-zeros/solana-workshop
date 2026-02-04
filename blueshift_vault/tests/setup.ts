import { LiteSVM } from "litesvm";
import { Keypair, PublicKey, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";
import * as path from "path";
import { PROGRAM_ID } from "./utils/constants";

// 程序 SO 文件路径
const PROGRAM_SO_PATH = path.join(__dirname, "../target/deploy/blueshift_vault.so");

/**
 * 创建并初始化 LiteSVM 实例，加载程序
 */
export function createSVM(): LiteSVM {
  const svm = new LiteSVM();
  svm.addProgramFromFile(PROGRAM_ID, PROGRAM_SO_PATH);
  return svm;
}

/**
 * 创建一个有资金的测试账户
 * @param svm - LiteSVM 实例
 * @param lamports - 空投的 lamports 数量
 * @returns 新创建的 Keypair
 */
export function createFundedAccount_v0(
  svm: LiteSVM,
  lamports: number = 10 * LAMPORTS_PER_SOL
): Keypair {
  const keypair = Keypair.generate();
  svm.airdrop(keypair.publicKey, BigInt(lamports));
  return keypair;
}

/**
 * 创建一个有资金的测试账户 (使用 setAccount)
 * @param svm - LiteSVM 实例
 * @param lamports - 账户余额 (lamports)
 * @returns 新创建的 Keypair
 */
export function createFundedAccount(
  svm: LiteSVM,
  lamports: number = 10 * LAMPORTS_PER_SOL
): Keypair {
  const keypair = Keypair.generate();
  svm.setAccount(keypair.publicKey, {
    lamports,
    data: new Uint8Array(0),
    owner: SystemProgram.programId,
    executable: false,
  });
  return keypair;
}

/**
 * 获取账户余额
 * @param svm - LiteSVM 实例
 * @param pubkey - 账户公钥
 * @returns 余额 (lamports)，如果账户不存在则返回 0
 */
export function getBalance(svm: LiteSVM, pubkey: PublicKey): bigint {
  return svm.getBalance(pubkey) ?? BigInt(0);
}
