
task2

https://learn.blueshift.gg/zh-CN/challenges/anchor-vault

使用了 surfpool 测试


## 关于program id 怎么保证一致？

Anchor 里“权威来源”通常是 **`target/deploy/blueshift_anchor_vault-keypair.json` 对应的公钥**（也就是部署用的 program keypair）。保证一致的常见做法是用 Anchor 的 keys 工具把它们同步：

- **查看**：`anchor keys list`
- **同步**：`anchor keys sync`

`anchor keys sync` 的作用就是：用部署 keypair 的公钥作为基准，把
- `programs/.../src/lib.rs` 里的 `declare_id!()`  
- 以及 `Anchor.toml` 里的 `[programs.<cluster>]`  
对齐到同一个 program id。

另外一个“被动保证”的机制是：**你部署到哪个 Program ID，链上就认哪个**；但客户端要能正确调用，仍然必须把 `Anchor.toml` / 生成的 TS 客户端里用到的 `programId` 配成同一个值，所以还是建议用 `anchor keys sync` 这种方式来统一。