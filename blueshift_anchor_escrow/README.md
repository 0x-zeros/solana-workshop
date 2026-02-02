

https://learn.blueshift.gg/zh-CN/challenges/anchor-escrow


警告
SPL Token-2022 的某些扩展功能，例如转账钩子、保密转账、默认账户状态，可能会引入漏洞，例如阻止转账、锁定资金以及在托管逻辑、金库或 CPI 中导致资金被抽走。

确保 mint_a 和 mint_b 由同一个代币程序拥有，以防止 CPI 失败。

使用经过充分审计的代币（例如 USDC、wSOL）来自标准 SPL Token 程序。

避免使用未经验证或复杂的 Token-2022 铸币。