# 上游同步作業指南 (UPSTREAM.md)

本文件規範 `SanHsien/agent-desktop` 如何安全地追蹤並合併上游 [`vercel-labs/agent-desktop`](https://github.com/vercel-labs/agent-desktop) 的變更。

---

## 1. Remote 配置與狀態確認

確認本地 Git 倉庫的 remote 正確配置：

```powershell
git remote -v
# 預期輸出：
# origin    https://github.com/SanHsien/agent-desktop.git (fetch)
# origin    https://github.com/SanHsien/agent-desktop.git (push)
# upstream  https://github.com/vercel-labs/agent-desktop.git (fetch)
# upstream  https://github.com/vercel-labs/agent-desktop.git (push)

# 確認 gh CLI 預設倉庫
gh repo set-default --view
# 必須為 SanHsien/agent-desktop
```

---

## 2. 上游變更拉取與比對

```powershell
# 1. 抓取上游最新 commit（不抓 tag：上游 tag 不帶進 fork，比對上游 tag 用 git ls-remote upstream）
git fetch upstream --no-tags

# 2. 檢視上游相較於本地 main 的新增 commit 清單
git log --oneline HEAD..upstream/main

# 3. 檢查差異檔案清單
git diff --stat HEAD..upstream/main
```

---

## 3. 合併與衝突處理

```powershell
# 1. 確保當前在乾淨的 main 分支
git checkout main
git status

# 2. 合併 upstream/main
git merge upstream/main

# 3. 若有衝突：
# - 上游程式碼檔案（crates/、src/、npm/）：先看衝突是否落在本 fork 的 Windows 修正（8d372752、0f7e8e65、a67bf683）；上游已有等效修正才取上游，否則保留，衝突解決後在 Windows 重測。
# - 本 fork 專屬維護檔案（FORK.md、NOTICE.md、docs/agent-desktop-computer-use-spec.md、tools/、docs/fork/）：保留本地版本。

# 4. 執行本地門禁驗證
pwsh -NoProfile -File tools/dev_check.ps1

# 5. 推送至個人 Fork（絕不推至 upstream）
git push origin main
```

---

## 4. 上游檢查器與水位

每週一的 `Upstream check` workflow 與本機的 `python tools/check_upstream_updates.py --strict` 讀同一份 `tools/upstream_baseline.json`，列出水位之後上游的 commit、PR（含已關閉）與 issue。

| exit code | 意義 |
| :--- | :--- |
| 0 | 完整檢查、沒有待審 |
| 1 | `--strict` 且有待審 |
| 2 | 檢查沒做完（`git`／`gh` 失敗、清單到上限還沒碰到水位、上游改寫歷史）；**不是**「沒有新東西」 |

- `track: release` 加 `ticket_cutoff: release`：上游幾天就有數十筆 PR／issue，只在新 release 出現時才要求審查；release 之後才開的項目留到下一輪。
- `reviewed_through`／`reviewed_release`：已審到的上游 release。2026-11-02 起點是 `v1.0.2`（`a47ed664`）；2026-11-10 審並以 `git merge --no-ff` 併入 `v1.2.0`（`2b0a7334`），本 fork 是 `2b0a7334` 加自己的 commit（ADR-009）；release 之後的上游 main commit 留給下一個 release。
- `reviewed_pr_through`（#569）、`reviewed_issue_through`（#577）：2026-11-10 檢查器列出的上限，結論逐項寫在 [`DECISIONS.md`](DECISIONS.md) ADR-009（上一輪 #476／#465 見 ADR-007）；不是「沒看過就設成最大號」，也不得高於檢查器報告寫的上限，否則 release 之後開的項目會被永久隱藏。
- `monitored` 釘住 PR #456、#400（ADR-007）與 #530、#543、#483、#561、#560（ADR-009）審過的 head 與狀態；漂移就轉紅，回對應 ADR 重看。
- 審完在 `DECISIONS.md` 記結論，再推進 `tools/upstream_baseline.json`。
- 不改檢查器程式；要改行為改艦隊正本（`repo-fleet-ops` 的 `tools/canonical/check_upstream_updates.py`）。
- 檢查器只 fetch 上游、只讀 `gh`，從不 push，也不對上游開 PR 或 issue。
