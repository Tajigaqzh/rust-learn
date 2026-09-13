# rust-learn

按章节由浅入深学 Rust 的笔记仓库：每一章配一份 Markdown 正文和一个可以直接 `cargo run` 的示例模块，正文里的结论都来自实际编译与运行结果。

在线文档：<https://tajigaqzh.github.io/rust-learn/>

## 目录结构

```text
src/                每章一个模块，形如 rustNN_主题/mod.rs
docs/guide/         每章一份正文，形如 rustNN-主题.md
docs/.vitepress/    文档站配置（VitePress）
.github/workflows/  GitHub Actions 工作流
```

## 环境要求

- Rust：edition 2024（rustc 1.85 及以上）
- Node.js 20+ 与 pnpm（只在构建文档站时需要）

## 常用命令

```bash
# Rust 示例代码
cargo run            # 依次运行第 1 章到最新一章的演示
cargo build          # 只编译
cargo doc --open     # 查看源码里的文档注释

# VitePress 文档站
pnpm install         # 安装依赖
pnpm docs:dev        # 本地预览 http://localhost:5173/rust-learn/
pnpm docs:build      # 构建静态站点到 docs/.vitepress/dist
pnpm docs:preview    # 预览构建结果
```

## 章节进度

| 章节 | 主题 | 正文 | 配套代码 |
| --- | --- | --- | --- |
| 第 1 章 | 打印输出 | [rust01-print](docs/guide/rust01-print.md) | `src/rust01_print/` |
| 第 2 章 | 变量与基本类型 | [rust02-variables](docs/guide/rust02-variables.md) | `src/rust02_variables/` |
| 第 3 章 | 函数与表达式 | [rust03-functions](docs/guide/rust03-functions.md) | `src/rust03_functions/` |
| 第 4 章 | 控制流 | [rust04-control-flow](docs/guide/rust04-control-flow.md) | `src/rust04_control_flow/` |
| 第 5 章 | 所有权与借用 | 编写中 | 编写中 |
| 第 6 章起 | 结构体、集合、错误处理…… | 规划中 | 规划中 |

完整路线见[学习路线与章节规划](docs/guide/index.md)。

## 文档站部署

推送到 `main` 分支后，GitHub Actions 会自动构建 VitePress 并发布到 GitHub Pages，工作流见 [.github/workflows/deploy.yml](.github/workflows/deploy.yml)。

站点地址：<https://tajigaqzh.github.io/rust-learn/>

因为是部署在项目子路径下，[docs/.vitepress/config.mts](docs/.vitepress/config.mts) 里设置了 `base: '/rust-learn/'`。以后如果改了仓库名、或者换成 `用户名.github.io` 这种用户主页仓库，记得把这里同步改掉（用户主页仓库要写成 `base: '/'`）。

首次部署前，需要在仓库的 Settings → Pages 里把 Source 选成 **GitHub Actions**（工作流里的 `configure-pages` 也带了自动启用，如果它没有权限，就需要手动点一下）。
