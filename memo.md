
[Surfpool 101 教程](https://learn.blueshift.gg/zh-CN/courses/testing-with-surfpool/surfpool-101)

**启动 Surfnet**
```bash
surfpool start
```

**测试命令**
```bash
anchor test --skip-local-validator
```


**用 surfpool 时不要让 Anchor 再起一个 validator**

1. 先启动 surfpool（保持运行）。
2. 在项目里用「跳过本地 validator」的方式跑测试，这样会**把程序部署到当前 RPC（即 surfpool）**，再跑测试：

```bash
anchor test --skip-local-validator
```

或使用已加好的脚本：

```bash
yarn test:surfpool
```