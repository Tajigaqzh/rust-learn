# workspace 示例（第 24 章）

一虚拟清单 + 两个成员的迷你工作区：

```text
examples/workspace-demo/
├── Cargo.toml          虚拟清单：只声明成员，不编译任何东西
├── Cargo.lock          工作区共享一份锁文件
├── core-lib/           业务规则（库）+ 单元测试
└── cli-bin/            命令行入口（二进制），依赖 core-lib
```

```bash
cd examples/workspace-demo
cargo build                  # 一次编译两个成员
cargo test                   # 跑工作区里所有测试
cargo tree                   # 看成员之间的依赖关系
cargo test -p core-lib       # 只跑指定成员
cargo run -p cli-bin         # 运行指定成员的二进制
```

这个目录有自己的 `Cargo.toml`，所以它**不是**根目录 `rust-learn` 包的一部分：
在仓库根目录执行 `cargo run` / `cargo build` 时它完全不参与。
