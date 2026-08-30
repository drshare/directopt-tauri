# directopt-desktop

绿电直连新能源优化配置软件（Tauri 2 + Vue 3 + TypeScript）。

## 本地开发

```bash
pnpm install
pnpm dev          # 仅前端
pnpm tauri dev    # 桌面端
pnpm tauri android dev
pnpm tauri ios dev
```

常用脚本：

| 命令 | 说明 |
| --- | --- |
| `pnpm typecheck` | `vue-tsc --noEmit` 类型检查 |
| `pnpm test` | Vitest 单元测试 |
| `pnpm build` | 类型检查 + 打包前端到 `dist/` |
| `pnpm tauri build` | 打包桌面端安装包 |

## CI / CD

仓库已迁移到 GitHub，流水线全部使用 GitHub Actions（Gitea 流水线已移除）：

- `.github/workflows/ci.yml`：PR 与 `main` 推送触发，只跑类型检查、单元测试、前端构建和 `cargo check`，快速反馈。
- `.github/workflows/release.yaml`：推送 `v*` 标签触发全平台矩阵构建（Linux / Windows / macOS / Android / iOS），生成校验和并发布 GitHub Release；也可手动触发，只构建上传产物、不发布 Release。
- `.github/actions/setup-env`：CI 与 Release 共用的环境准备（pnpm / Node / Rust 与缓存）。

发布流程：

```bash
git tag v0.0.1
git push origin v0.0.1
```

### 可选 Secrets

不配置时流水线仍可运行，只是产物未签名。

| 名称 | 用途 |
| --- | --- |
| `ANDROID_KEYSTORE_BASE64` / `ANDROID_KEYSTORE_PASSWORD` / `ANDROID_KEY_ALIAS` | Android APK 签名与 AAB 构建 |
| `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` / `APPLE_SIGNING_IDENTITY` | macOS 签名 |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | macOS 公证 |

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
