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
