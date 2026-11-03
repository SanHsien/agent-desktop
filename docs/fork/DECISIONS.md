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
  2. `reviewed_pr_through` 與 `reviewed_issue_through` 從 0 開始，分批審完才提高；不把它們設成上游目前最大號。**（已被 ADR-007 取代：審完後水位設為 PR #476、issue #465；這一點保留作當時判斷的紀錄。）**
  3. 檢查器用艦隊標準版（目前與 `repo-fleet-ops` 的正本同版，sha1 `c484ddbc…`；正本更新後再換），不在本 fork 改程式；差異只放 baseline（`decision_log` 指向本檔）。
  4. 不開上游分支軸：上游有 177 個分支，落到 `main` 的內容會經 commit 軸抵達。

---

## ADR-006: 依賴新鮮度只讀 Cargo 與 Actions 的宣告

- **日期**：2026-11-02
- **狀態**：Accepted
- **脈絡**：Dependabot 看的是 lock 的移動，不回答「宣告的下限是否悄悄變舊」。本 fork 的 `npm/package.json` 沒有任何依賴，`apps/docs` 是上游的文件站，本 fork 不部署（`gh api repos/SanHsien/agent-desktop/deployments` 為 0、Pages API 回 404），不讀它的理由只有這一條。Dependabot 版本更新沒涵蓋它（`.github/dependabot.yml` 只有 cargo、npm `/npm`、github-actions），但安全更新是開的：2026-11-02 `apps/docs/pnpm-lock.yaml` 有 4 筆 open alert（minimatch、glob 為 medium，katex、esbuild 為 low），`296ef839` 上 4 個 `npm_and_yarn in /apps/docs` 更新 job 全 failure（log 只有 updater error，細節需要 repo 寫入權限）。處理結果見下方決策第 5 點。
- **決策**：
  1. `tools/check_dependency_freshness.py` 讀根目錄 workspace 與各成員的 `Cargo.toml`（含 target 限定的依賴）對 crates.io，以及 `.github/workflows/*.yml` 的 action 釘版對 GitHub Releases。
  2. 紅燈只有兩條出口：宣告行的 `# freshness-hold:`（常態政策），或 `.github/dependency-deferrals.json`（已評估、這個月不升，超過 `deferredLatest` 自動失效）。調高下限來消音不算。
  3. 目前的 deferral 共 9 筆，每筆的 `deferredLatest` 是 2026-11-02 讀 crates.io／GitHub Releases 的當日最新版，registry 超過它就自動失效。`zbus`：`crates/linux` 釘 `=5.19.0`，`deny.toml` 記載 5.19 是有 blocking API 的最新版；理由取自 `deny.toml`，未對照 zbus 6 changelog。`futures-util`（`=0.3.33`，最新 0.3.34）、`rustix`（`=1.1.5`，最新 1.1.6）也是 `crates/linux/Cargo.toml` 的精確釘版。`actions/download-artifact`、`setup-node`、`setup-python`、`upload-artifact`、`github/codeql-action/init`、`analyze` 是 `ci.yml`、`codeql.yml`、`release*.yml`、`supply-chain.yml`、`windows-capability-probe.yml` 的釘版。後兩類沒有 `deny.toml` 那樣的專屬理由，擱置的依據只有一條：宣告所在的檔案與上游 `main` 逐字相同（`git diff upstream/main HEAD --stat -- crates/linux .github/workflows` 只列出本 fork 自己的 4 個 workflow，沒有 `crates/linux`；`release.yml` 的差異是 ADR-004 的一行 guard，不在釘版行）。
  4. 上游持有檔案裡落後的釘版（第 3 點列的 8 筆非 `zbus` 項目）不在本 fork 改，也不合併動到這些檔案的 Dependabot PR（合併它與手改一樣會在日後上游同步時衝突）；等上游升級後經 commit 軸同步，等待期間以第 3 點的 deferral 擱置，並由 `ROADMAP.md` 追蹤。fork 的 Dependabot version updates 是否開啟尚未實查，不作為出口。
  5. `apps/docs` 的 Dependabot 安全告警（2026-11-02 的 #1 esbuild、#2 katex、#3 glob、#4 minimatch）於 2026-11-03 以 `tolerable_risk` 駁回：該站是上游的文件站，本 fork 不建置、不部署（deployments 0、Pages 404），四個安全更新 job 皆為 `security_update_not_possible`（可解析版本上限低於修補起點），手動升級會改到上游持有的 `apps/docs` 而在同步時衝突。重新評估條件：本 fork 開始建置或部署 `apps/docs`，或上游自行升級到修補版本後經 commit 軸同步。

---

## ADR-007: 第一次上游審查與維護工具（2026-11-02）

- **日期**：2026-11-02
- **狀態**：Accepted
- **取代**：ADR-005 第 2 點（PR／issue 水位從 0 起算）。ADR-005 第 1 點的 commit 軸起點 `9eb9b296` 仍成立，但 baseline 的 `reviewed_through` 改記 release `a47ed664`（`track: release`）；第 3、4 點不變。
- **快照**：上游 `main` `9eb9b296`（本 fork 的 merge base，behind 0）；release `v1.0.2`／`a47ed664`；PR 381 筆（merged 239、open 61、closed 未合併 81）；issue 81 筆（open 41、closed 40）；以 `state=all` 讀 API，水位 PR #476、issue #465。
- **方法**：本 fork 與上游差 54 個檔案（`git diff --name-only upstream/main HEAD`，2026-11-02 在 `main` `296ef839`），其中 32 個是 `crates/`、`src/` 的產品檔，不能用「fork 與上游相同」的結構性論證。142 筆未合併 PR 逐筆取檔案清單，和差異檔比對，再讀 Windows 相關與有重疊的項目。已合併但 base 不是 `main` 的 82 筆另依 head 分支追到收口 PR（見表中一列），沒有逐筆比對檔案。同一批 diff 掃隱藏內容（長空白夾內容、`eval(`、`new Function(`、超長 base64）：命中 #442、#365、#226、#170、#60、#53，看過都是 C 標頭、Objective-C 續行或 Markdown 表格的對齊空白，沒有隱藏內容。

| 項目 | 判定 | 證據 | 重看條件 |
| --- | --- | --- | --- |
| commit 軸（到 `9eb9b296`） | 已含 | merge base 即上游 HEAD，無未併入 commit | 新 release |
| #440（已關閉）桌面附掛、EnumWindows 回 0 | 不採用，做法已在 | +46/-3；本 fork `crates/windows/src/system/window/window_enum.rs:81` 已呼叫 `attach_input_desktop()`，`:91-94` 已分辨 `GetLastError`。沒有照抄它退回 `default` 桌面的分支 | Agent 子 shell 又出現 `EnumWindows failed` |
| #367、#366（已關閉）Windows 游標 PNG、caret | 不採用 | 作者自述只跑 clippy、沒在 Windows 跑；與本 fork 只重疊 `crates/windows/Cargo.toml`；同串 #362–#365 同理 | 實際遇到游標圖不更新或 caret 殘留 |
| #456（open）npm 二進位啟動游標 overlay | 監看，釘 head | +32/-2；對應 issue #422（回報者與維護機同為 build 26300）。本機未重現 | 合併、或 head 變動 |
| #400（open）背景 Chromium／Electron 無頭輸入 | 監看，釘 head | +1028/-10，15 個 Windows 檔；對應 issue #395，符合本 fork 用途，但改動大且未合併 | 合併、或 head 變動 |
| #457（open）wait 事件的 hosted app 歸屬 | 等合併 | +11/-2，只動 `signal_inventory.rs`，無重疊 | 下次 release 審查 |
| #442（open，取代已關閉的 #427、#429）身分解析重構 | 等合併 | +7241/-6139；與本 fork 重疊 `hit_test/envelope_tests.rs`、README、AGENTS，合併時會衝突 | 合併後第一次同步 |
| #411（open）私有狀態目錄拒絕存取回 PERM_DENIED | 等合併 | +35/-5，與本 fork 的 `session/tree.rs` 重疊 | 下次 release 審查 |
| 其餘有重疊者：README／AGENTS／`release.yml`／`crates/ffi` 的文件、依賴 bump 與已關閉的舊 PR | 不適用 | 重疊只在本 fork 重寫的文件或已加 guard 的 workflow；#60、#53 已關閉，動到 `crates/ffi` 的只有舊版內容，沒有逐行比對 | 改 `crates/ffi` 時 |
| 其餘未合併 PR（多為 macOS／Linux、CLI 文件） | 不適用或等合併 | 不碰本 fork 差異檔；本 fork 只發 Windows 版 | 下次 release 審查 |
| issue #459（Notepad 無頭輸入被拒） | 監看 | 無 PR；本機未重現 | 上游修復或本機遇到 |
| issue #417（ARM64 結束時崩潰） | 不適用 | 本 fork 只發 x86_64 | 改發 ARM64 |
| 其餘 issue | 不適用或等上游 | macOS／Linux 專屬為主；只用標題分類 | 下次 release 審查 |
| 已合併但 base 不是 `main` 的 PR（82 筆；標題含 `windows`／`win32`／`uia` 的 30 筆，#445、#447 在 baseline 舊 notes 被點名） | 55 筆內容已隨 commit 軸進入；25 筆不適用；2 筆（#445、#447）等合併 | 2026-11-02 唯讀 `gh pr list --state all` 依 head 分支追堆疊。**55 筆**（含標題含 windows 的 28 筆）的最底層收口 PR 已合進 `main`，merge commit 是 `upstream/main` 的祖先：`feat/windows-adapter` 的 32 筆經 #110（`483a31d0`），`ctate/*` 經 #342、#345、#347，其餘經 #67、#77、#93、#137；沒有逐筆驗證堆疊內每個 commit 都在該 merge 之前進入分支。**25 筆**在 `feat/linux-adapter` 的堆疊，收口 PR #298 已關閉未合併；其中標題含 windows 的 #290、#305 內容是 Linux 轉接層，本 fork 只發 Windows 版。**2 筆**（#445、#447）在 `laf/identity-linux-windows`，收口 PR #442 開著，且 base 是 `laf/identity-lean`（其上還有開著的 #441，base `laf/remove-macos-host`），不是 `main`；後面的鏈沒有追完 | #442 合併進 `main`、#298 重開或其分支進 `main`、或 `Upstream check` 在新 release 後列出相關項目 |
| 分支軸 | 不開 | API 列出 177 個上游分支；本輪未逐一比對未開 PR 的分支 | 上游開長期分支 |

- **產品差異清單**：逐檔理由、移除條件與比對方式見 [DIVERGENCE.md](../DIVERGENCE.md)；`tools/check_divergence.py` 核對 `crates/`、`src/`、`apps/`、`npm/`，漏列或多列會失敗。
- **水位軌道**：`track: release` 加 `ticket_cutoff: release`。理由見 [`UPSTREAM.md`](UPSTREAM.md) 第 4 節。
- **依賴新鮮度**：另立 [ADR-006](#adr-006-依賴新鮮度只讀-cargo-與-actions-的宣告)。同一天的實查結論：Cargo 與 Actions 共 9 筆落後的新版本屬上游持有的宣告，寫成 `.github/dependency-deferrals.json`。**`apps/docs`** 不讀，理由只有一條：上游的文件站，本 fork 不部署（`gh api repos/SanHsien/agent-desktop/deployments` 為 0、Pages API 回 404）。Dependabot 版本更新沒涵蓋它（`.github/dependabot.yml` 只有 cargo、npm `/npm`、github-actions），但安全更新是開的：4 筆 open alert、4 個失敗的更新 job；2026-11-03 已駁回，見 ADR-006 第 5 點。
- **已知問題：`v1.0.2` tag 與上游同名不同物**。本 fork 的 tag 指向 `5d82883d`（fork 自己的 release，只有 Windows x64），上游是 `a47ed664`。`5d82883d` 的 `crates/`、`src/`、`apps/`、`npm/` 與上游 `main` `9eb9b296`（`a47ed664` 之後的內容）相同，與上游的 `v1.0.2` 差 93 個產品檔（`git diff --name-only 5d82883d a47ed664 -- crates src apps npm`）。`git fetch upstream --tags` 可能因此拒絕更新 tag，所以 [`UPSTREAM.md`](UPSTREAM.md) 改用 `--no-tags`。2026-11-02 已決定改名，見 ADR-008。

---

## ADR-008: fork 的 release 用 `fork-` 前綴

- **日期**：2026-11-02
- **狀態**：Accepted
- **脈絡**：fork 的 `v1.0.2` tag 指向 `5d82883d`，上游的 `v1.0.2` 是 `a47ed664`，同名不同物，讀版本號的人會以為兩者相同。沒有文件或工具引用舊的下載網址（查過本 repo、`ai-inbox`、`agent-assets`）。
- **決策**：release 與 tag 改名 `fork-v1.0.2`，刪除 fork 上的 `v1.0.2` tag；commit、資產與內容不變。不把 `v1.0.2` 強推回上游的 commit：要比對上游 tag 用 `git ls-remote upstream`。之後 fork 的 release 一律用 `fork-` 前綴。維護者授權決定。
- **代價**：`releases/tag/v1.0.2` 與 `releases/download/v1.0.2/*` 的舊連結失效。
