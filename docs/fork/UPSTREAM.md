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
# 1. 抓取上游最新 commit 與 tags
git fetch upstream --tags

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
# - 上游程式碼檔案（crates/、src/、npm/）：一律以上游最新為準，衝突解決後重測。
# - 本 fork 專屬維護檔案（FORK.md、NOTICE.md、docs/agent-desktop-computer-use-spec.md、tools/、docs/fork/）：保留本地版本。

# 4. 執行本地門禁驗證
pwsh -NoProfile -File tools/dev_check.ps1

# 5. 推送至個人 Fork（絕不推至 upstream）
git push origin main
```
