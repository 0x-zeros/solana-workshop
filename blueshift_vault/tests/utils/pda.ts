import { PublicKey } from "@solana/web3.js";
import { PROGRAM_ID, VAULT_SEED } from "./constants";

/**
 * 派生 Vault PDA
 * @param owner - Owner 的公钥
 * @returns [vaultPda, bump]
 */
export function deriveVaultPDA(owner: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from(VAULT_SEED), owner.toBuffer()], PROGRAM_ID);
}
