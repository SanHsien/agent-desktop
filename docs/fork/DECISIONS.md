# 架構決策記錄 (Architecture Decision Records)

本文件記載 `SanHsien/agent-desktop` 的關鍵架構決策（ADR）。

---

## ADR-001: 維持完整 Git 歷史與雙 Remote 架構

- **日期**：2026-11-01
- **狀態**：Accepted
- **脈絡**：需要維護個人 fork 並隨時跟進 Vercel Labs 上游更新，同時避免誤推上游或遺失 commit 追溯力。
- **決策**：
  1. 完整保留 upstream Git 歷史，不採 squash / orphan 獨立歷史。
  2. 設定 `origin` 指向 `SanHsien/agent-desktop`，`upstream` 指向 `vercel-labs/agent-desktop`。
  3. 強制使用 `gh repo set-default SanHsien/agent-desktop` 釘死 CLI 目標，防止 `gh pr create` 或其他指令誤打上游。

---

## ADR-002: Windows UIA 無障礙樹與 PowerShell 單引號防雷規範

- **日期**：2026-11-01
- **狀態**：Accepted
- **脈絡**：
  - Windows 11 下，Agent 操作桌面若依賴視覺截圖，Token 成本極高且容易座標漂移。
  - 在 PowerShell 下，`@ref`（如 `@s8f3k2p9:e12`）會觸發 PowerShell 的 Splatting 展開運算子，未加引號的參數會被吞噬導致命令失敗。
- **決策**：
  1. 將 Windows UI Automation 原生無障礙樹作為首選操控途徑。
  2. 在全域規範、Prompt 範本與技能說明中強制要求：所有傳入命令列的 Ref 必須以單引號包裹（`'@s8f3k2p9:e12'`）。
  3. 實作 `tools/verify_a11y_tree_snapshot.py` 驗證 Ref 解析器與剝離邏輯。

---

## ADR-003: 全系統跨 Agent 技能發布與 SSOT 整合

- **日期**：2026-11-01
- **狀態**：Accepted
- **脈絡**：維護者擁有 Antigravity、Claude Code、Codex、Hermes 四大主力 Agent，必須讓所有 Agent 能無差別調用 `agent-desktop`。
- **決策**：
  1. 在 `SanHsien/agent-assets` 登記核准技能 `skills/agent-desktop/SKILL.md` 與知識文件 `knowledge/agent-desktop-os-accessibility-tree.md`。
  2. 發布至全域技能目錄 `~/.gemini/config/skills/agent-desktop/SKILL.md`。
  3. 本地原生二進制預先安裝並校驗 SHA-256 Checksum（`npm/bin/agent-desktop-win32-x64.exe`），提供毫秒級無延遲呼叫能力。

---

## ADR-004: GitHub Workflows 發布防護加固

- **日期**：2026-11-02
- **狀態**：Accepted
- **脈絡**：上游 `release.yml` 與發布作業在 main 分支更新時會自動觸發，在 fork repo 上執行可能因缺少官方 Secrets 或權限失敗。
- **決策**：
  在 `.github/workflows/release.yml` 等發布 workflow 中加入 `if: github.repository == 'vercel-labs/agent-desktop'` 守衛條件，確保 fork 上的日常維護推送不會觸發意外的 npm 或 release 構建失敗。

---

## ADR-005: 上游水位的起點與檢查器來源

- **日期**：2026-11-02
- **狀態**：Accepted
- **脈絡**：本 fork 納入艦隊治理時，需要一個誠實的「已審到哪」。實查（2026-11-02）：本 fork 是 `upstream/main` `9eb9b296` 加 7 個自己的 commit（`git rev-list --left-right --count HEAD...upstream/main` 為 `7 0`）；上游 PR 381 筆、issue 81 筆從未審過；上游會把堆疊 PR 合進非 `main` 的分支（#445、#447 合進 `laf/identity-linux-windows`），所以已合併的 PR 號碼不能當成「已在本 fork」的證據。
- **決策**：
  1. `reviewed_through` 設在分叉點 `9eb9b296`（commit 軸沒有待審）。
  2. `reviewed_pr_through` 與 `reviewed_issue_through` 從 0 開始，分批審完才提高；不把它們設成上游目前最大號。
  3. 檢查器用艦隊標準版（逐位元組與 `repo-fleet-ops` 的正本相同），不在本 fork 改程式；差異只放 baseline（`decision_log` 指向本檔）。
  4. 不開上游分支軸：上游有 177 個分支，落到 `main` 的內容會經 commit 軸抵達。

---

## ADR-006: 依賴新鮮度只讀 Cargo 與 Actions 的宣告

- **日期**：2026-11-02
- **狀態**：Accepted
- **脈絡**：Dependabot 看的是 lock 的移動，不回答「宣告的下限是否悄悄變舊」。本 fork 的 `npm/package.json` 沒有任何依賴，`apps/docs` 是上游的文件站（本 fork 不建置、不部署，Dependabot 也不掃）。
- **決策**：
  1. `tools/check_dependency_freshness.py` 讀根目錄 workspace 與各成員的 `Cargo.toml`（含 target 限定的依賴）對 crates.io，以及 `.github/workflows/*.yml` 的 action 釘版對 GitHub Releases。
  2. 紅燈只有兩條出口：宣告行的 `# freshness-hold:`（常態政策），或 `.github/dependency-deferrals.json`（已評估、這個月不升，超過 `deferredLatest` 自動失效）。調高下限來消音不算。
  3. 目前唯一的 deferral 是 `zbus`：`crates/linux` 釘 `=5.19.0`，`deny.toml` 記載 5.19 是有 blocking API 的最新版；理由取自 `deny.toml`，未對照 zbus 6 changelog。
  4. 上游持有檔案裡落後的釘版（`futures-util`、`rustix`、數個 action）不在本 fork 改，也不合併動到這些檔案的 Dependabot PR（合併它與手改一樣會在日後上游同步時衝突）；等上游升級後經 commit 軸同步。等待期間可用 `.github/dependency-deferrals.json` 擱置（`deferredLatest` 填當日最新版，reason 寫 upstream-owned），其餘留紅燈由 `ROADMAP.md` 追蹤。fork 的 Dependabot version updates 是否開啟尚未實查，不作為出口。
