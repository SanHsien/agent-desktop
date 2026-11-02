# Fork 維護說明

本 repo fork 自 [`vercel-labs/agent-desktop`](https://github.com/vercel-labs/agent-desktop)，沿用 Apache-2.0 授權與完整 Git 歷史。

---

## 1. 為什麼維護 fork

1. **全系統 AI Agent 桌面操作落地**：
   - 為本機 Windows 11 環境上的四大主力 Agent（Antigravity、Claude Code、Codex、Hermes）提供標準化的 OS 級無障礙樹（Accessibility Tree）操作工具與執行環境。
   - 揚棄傳統視覺截圖方案（每步 1,000~1,800 tokens、高延遲、座標易漂移），改以直接存取作業系統底層無障礙樹之語意節點（每步 80~200 tokens，骨架模式低於 50 tokens，Token 節流達 90%~95%）。
2. **Windows 11 原生加固與防雷規範**：
   - 實證並記錄 Windows WindowStation / Desktop 隔離切換深層機制（`OpenDesktop` / `SetThreadDesktop`）。
   - 制定 PowerShell `@ref` splatting 展開防呆語法（強制單引號包裹 `'@ref'`）。
   - 嚴格落實 `STALE_REF` 狀態機與處置重試安全合約（Disposition Retry Contract），杜絕 GUI 自動化常見的 Doom Loop 無限重試。
3. **保持與上游同步**：
   - 完整追蹤 `upstream/main`（Rust Core 與平台介面更新），在保留上游所有原生能力的同時，建立個人維護與品質保證體系。

**回貢判準**：
- 修的是上游 Rust Core、平台 Adapter 或 CLI 的 bug，經本地驗證後提 PR 回貢上游 `vercel-labs/agent-desktop`。
- 本 fork 獨創之跨 Agent 規格書、全域 Skill 整合、Windows PowerShell 維護骨架與 CI 衛生門禁則留在本 fork。
- 回貢前必須在當次對話取得維護者（主人）明確同意；「fork」、「建環境」均不代表同意。

---

## 2. 與上游的差異

| 項目 | 說明 |
| :--- | :--- |
| `FORK.md`、`NOTICE.md` | Fork 來源、授權聲明與維護規則 |
| `docs/agent-desktop-computer-use-spec.md` | OS 級 Computer Use 架構規格書（a11y vs Pixel 矩陣、UIA 映射、Ref 演算法、Disposition 合約） |
| `docs/fork/UPSTREAM.md` | 上游同步、分支管理與變更審查手冊 |
| `docs/fork/DECISIONS.md` | 架構決策記錄（ADRs） |
| `tools/dev_check.ps1` | Windows 本機一鍵驗證門禁腳本（Git 設定、檔案編碼、文件完整性、衛生測試） |
| `tests/fork/` | 本 fork 維護工具與衛生檢查單元測試 |
| `.github/workflows/fork-maintenance.yml` | 本 fork 專屬之 CI 流程（文件格式、連結、衛生門禁） |
| `.github/workflows/release.yml` | 加上 `if: github.repository == 'vercel-labs/agent-desktop'` 防護，防止在 fork 上意外觸發上游發布作業 |
| `.cursor/rules/no-upstream-pr.mdc` | 本地編輯器防呆：防止誤對上游倉庫開 PR 或推送 |
| `ROADMAP.md` | 本 fork 之後要做什麼的單一真相源 |
| `tools/check_upstream_updates.py`、`tools/upstream_baseline.json`、`.github/workflows/upstream-check.yml` | 每週列出上游水位之後的新 commit、PR、issue；檢查器為艦隊標準版，差異只放 baseline |
| `tools/check_dependency_freshness.py`、`.github/dependency-deferrals.json`、`.github/workflows/dependency-freshness.yml` | 每月比對 Cargo 與 Actions 宣告的下限和現行版 |
| `tests/fork/test_fork_tools.py` | 上述兩支檢查器與其接線的性質測試 |

上游持有的程式碼（`crates/`、`src/`、`apps/`、`npm/`）原樣保留，上游主要文檔以英文為準。

---

## 3. 分支與 Remote 規則

- **`origin/main`**：`SanHsien/agent-desktop`，主要維護線。
- **`upstream`**：`vercel-labs/agent-desktop`，只追蹤、不推送。

### 開 PR 與推送的硬規則

日常修改與 PR **只能**打進 `SanHsien/agent-desktop`：

```powershell
gh repo set-default SanHsien/agent-desktop   # 每個 clone 先跑一次
gh repo set-default --view                   # 必須回 SanHsien/agent-desktop
git remote -v                                # 確認 origin 指向 SanHsien
```

嚴禁執行 `git push upstream` 或對 `vercel-labs/agent-desktop` 裸開 PR。

---

## 4. 本地開發與驗證

在 Windows 11 原生環境下驗證本 fork：

```powershell
# 1. 執行本地門禁驗證
pwsh -NoProfile -File tools/dev_check.ps1

# 2. 測試 agent-desktop 狀態
& "npm/bin/agent-desktop-win32-x64.exe" status
```
