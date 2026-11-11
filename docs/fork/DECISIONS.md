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
| 分支軸 | 不開 | API 列出 177 個上游分支；當時未逐一比對未開 PR 的分支 | 上游開長期分支 |

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

---

## ADR-009: 第二次上游審查，合併 v1.2.0（2026-11-10）

- **日期**：2026-11-10
- **狀態**：Accepted
- **觸發**：`Upstream check` 2026-11-09 failure；上游 2026-11-04 發 `v1.2.0`（`2b0a7334`，`git ls-remote upstream 'refs/tags/v1.*'`：`v1.1.0` 為 `d3dc190a`）。
- **快照**：`git fetch upstream --no-tags` 後 `upstream/main` 為 `ab7dfb21`；`git rev-list --left-right --count HEAD...upstream/main` 在審查前為 `19 20`（報告檔寫的 35／36 是相對 baseline `a47ed664` 的數字）。release 軌只審到 `v1.2.0`：release 之後的 4 個 main commit（`5e0491ae`、`b43ac4f6`、`77149fd3`、`ab7dfb21`）留給下一個 release。
- **commit 軸**：檢查器列 32 個 `a47ed664` 之後的 commit；其中 16 個（`0768da4` 至 `9eb9b29`）本來就是本 fork 的 merge base `9eb9b296` 的祖先（`git merge-base --is-ancestor <hash> HEAD` 逐一為真）。其餘 16 個是 `7b69db7`、`b4926b5`、`0dd484f`、`060c83a`、`31b757a`、`d3dc190`、`8b7fa26`、`93ae89e`、`17467da`、`87f4f41`、`6660fba`、`c463bce`、`53e072d`、`97125f6`、`d83bfaf`、`2b0a733`，共 `git diff --shortstat 9eb9b296 2b0a7334` 的產品、文件與依賴變更。
- **決策：合併 `2b0a7334`**（`git merge --no-ff`，不是挑揀）。理由與證據：
  1. 衝突面極小：本 fork 的 34 個產品檔與上游這 16 個 commit 的檔案集合只重疊 `crates/windows/src/tree/hit_test/envelope_tests.rs`（`comm -12` 兩邊的 `git diff --name-only`），該檔自動合併，fork 那一行（occluded 測試預算 80 改 1000 ms）仍在；文字衝突只有 `README.md`。`git merge-tree --write-tree HEAD 2b0a7334` 事先給出同樣結論。
  2. 逐檔差異集合不變：合併後 `git diff --name-only --no-renames 2b0a7334 -- crates src apps npm` 仍是 34 檔，和 `docs/DIVERGENCE.md` 相同，只把 Base 從 `9eb9b296` 改到 `2b0a7334`；`python tools/check_divergence.py` exit 0。
  3. 該 commit 範圍含 ROADMAP 中期列的觸發條件（#442 合進 `main`、上游發 1.1.0）。
- **README 處理**：`README.md` 是 fork 自己的中文版，衝突取 fork 側整份，不採上游改寫；`README.en.md` 是 fork 自己的檔，不受影響。依上游事實補了兩處：`agent-desktop mcp` 與 `--stdin` 的簡短說明（兩份 README 各一段），以及 `ad_init(6)` 改 `ad_init(7)`、範例 envelope `"version": "2.5"` 改 `"2.6"`，對照 `AD_ABI_VERSION_MAJOR`（標頭為 7）與 `ENVELOPE_VERSION`（`crates/core/src/response/output.rs:7`）。`ad_init(6)` 在合併後的 `cargo test -p agent-desktop` 使 `documented_abi_major_matches_the_header` 失敗（`README.md documents ABI major 6`，左 6 右 7），改後通過。
- **這次合併帶來的行為風險（未做 Windows live 驗證）**：#442 刪除 Windows 的 Chrome 修復與路徑回退解析（`crates/windows/src/tree/resolve/resolve_chrome_repair.rs`、`resolve_pair_window.rs` 及 6 個測試檔），改走上游的 capture rule。Windows lib 測試數因此從 1549 筆測試名（合併前，`1543 passed; 0 failed; 6 ignored`）變成 1488 筆（合併後 `1482 passed; 0 failed; 6 ignored`），減少的名稱全在 `tree::resolve::*`（`comm` 比對兩份 `cargo test` 輸出）。單元測試與 fixture 測試通過，但 Chromium／Electron 的 ref 重解析沒有人在真實桌面上跑過。**fork 不得在 live E2E 通過前發新的 fork release**（ROADMAP 有對應列）；維護者本機已安裝的 `npm/bin` 二進位不受這次原始碼合併影響。
- **PR 軸（#477 至 #569，63 筆）**：2026-11-10 唯讀 `gh pr list --state all --limit 400` 取得中繼資料，63 筆逐筆 `gh pr diff` 取回全文並對新增行掃隱藏內容（長空白夾內容、`eval(`、`new Function(`、200 字元以上 base64、零寬與方向控制字元）：命中 #490、#506、#528、#549，看過都是 Markdown／shell 註解對齊空白、Objective-C 續行對齊、檔首 BOM（#549 `scripts/jev/run.integration.test.mjs`），沒有隱藏內容。程式碼層級讀過的只有 #530、#536、#543（Windows 輸入、npm shim、靜態 CRT）、#531、#560、#483 的關鍵片段；其餘依標題加檔案清單分類，**沒有逐行讀**。9 筆合併進 `main`、4 筆合併進堆疊分支、49 筆開著、1 筆關閉未合併（#546，與 #547 同一 head `984f53e2`）。

| 項目 | 判定 | 證據 | 重看條件 |
| --- | --- | --- | --- |
| 已合併進 `main` 的 9 筆：#480、#481、#502、#503、#508、#511、#519、#524、#568 | 已併入（本次合併） | 對應 commit `31b757a`、`17467da`、`060c83a`、`8b7fa26`、`d3dc190`、`93ae89e`、`6660fba`、`87f4f41`、`2b0a733` 皆為 `2b0a7334` 的祖先；#480 動 Windows 3 檔、core 22 檔，#519 是 `agent-desktop mcp` | 下次 Windows live E2E |
| 合併進堆疊分支的 4 筆：#482、#525、#526、#564 | #482、#525：內容隨收口 PR #480、#519 進入；#526、#564：不適用 | #482、#525 的檔案路徑在 `2b0a7334` 都存在（不逐 blob 比對）；#526 收口的 #506 是 macOS VM sandbox 且開著；#564 只動 `crates/macos` 2 檔 | #506 合併；改發 macOS |
| #442（已合併）身分解析重構，帶入 #445、#447 | 已併入（本次合併），見上方風險 | ROADMAP 中期列「同步上游堆疊分支」的觸發條件成立；`0dd484f` 289 檔；#445 的 7 個、#447 的 100 個路徑全在 `2b0a7334` | Windows live E2E、或 Chromium ref 解析回報問題 |
| #530（open，head `6af4b796`）Windows 標點鍵與導覽鍵 extended 旗標 | 等合併，釘 head | 讀過程式碼：`keyboard_event.rs` 加 `KEYEVENTF_EXTENDEDKEY`、`keyboard_map.rs` +73、diff 新增 7 個 `#[test]`；對應 issue #513。fork 有 Windows `press`，但本機沒遇到 | 合併、head 變動、或本機遇到標點鍵被拒／Shift+方向鍵不選取 |
| #543（open，head `f6cdfb22`）靜態連結 MSVC runtime | 等合併，釘 head | 讀過程式碼：`.cargo/config.toml` 加 `+crt-static`、新增 PE 匯入檢查腳本；對應 issue #542。維護者本機已有 VC++ runtime；`.cargo/config.toml` 會改所有 Windows 建置，不在審查中順手改 | 合併、head 變動、或 fork 的 release 在乾淨機器無法啟動 |
| #483（open，head `a70d0177`）hosted app 的 list-apps／close-app | 等合併，釘 head | 讀過程式碼：`frame_identity::agent_facing_owner`；+179/-44，改 `window_ops.rs`、`close.rs`；與 issue #555 同一類 | 合併、head 變動 |
| #561（open，head `524c1d88`）深層觀測與 ref 讀取 deadline | 等合併，釘 head | +998/-63，28 檔含 `crates/windows/src/system/private_file/replace.rs`、`tree/walk/walker.rs`；對應 issue #558 | 合併、head 變動 |
| #560（open，head `e222c6b5`）Windows 儲存層 state-root 擁有者與 private-file 路徑穿越 | 等合併，釘 head | 讀過程式碼片段：`crates/core/src/storage/state_root.rs` +153、`private_file_parent.rs` +83，新增 `windows-sys` 依賴；PR 自稱安全修補，本次沒有重現 | 合併、head 變動 |
| #521、#565、#490、#531、#536 | 等合併 | #521 Chromium 工具列 popover（+371/-13，Windows 17 檔）；#565 選取文字（+3948，91 檔，19 個 Windows 檔）；#490 Dogwood 政策（+8879，100 檔，帶 `crates/policy` 與 Cargo.lock +1530）；#531 `type` 被拒時建議 `set-value`（core + skill）；#536 npm shim 相對路徑（讀過）。體量大或只影響 npm 安裝／文件，未見 fork 的現行問題 | 合併進 `main` |
| #517、#520、#549（`scripts/jev`） | 不適用 | jev 是上游範例腳本，fork 不使用；#549 的檔首 BOM 已確認無害 | fork 開始用 jev |
| 跨平台 core／CLI 修正：#477、#488、#489、#496、#497、#498、#500、#538、#547（#546 為其重複，已關閉）、#523 | 等合併 | 皆在 `crates/core` 或 `src/`、`npm/bin`，標題為競態與狀態讀取修正；不碰 fork 的 34 個差異檔 | 下次 release 審查 |
| macOS 專屬：#484、#485、#493、#495、#499、#501、#504、#505、#506、#528、#541、#544、#551、#553、#556、#566 | 不適用 | 動 `crates/macos`、`tests/native`、Objective-C；#528 有 1 行 `crates/windows/src/system/cursor_overlay/pipe_name.rs` 的重新命名，隨它合併 | 改發 macOS |
| Linux、文件、依賴、CI：#514、#515、#518、#522、#527、#535、#539、#540、#557、#563 | 不適用或等合併 | #515 是 `Cargo.toml` default-members 加 `AGENTS.md` 一行；#539、#540 是上游依賴 bump，依 ADR-006 隨上游同步 | 下次 release 審查 |

- **issue 軸（#478 至 #569，25 筆）**：只用標題與內文是否提到 Windows 分類，沒有逐筆重現。

| 項目 | 判定 | 證據 | 重看條件 |
| --- | --- | --- | --- |
| #513、#532、#542、#558 | 等對應 PR 合併 | 分別對應 #530、#536、#543、#561（open） | PR 合併，或本機遇到 |
| #478、#555、#487、#486 | 監看 | Windows 視窗與 app 身分／最小化視窗的回報；#478 回報環境是 Windows 10，本機是 Windows 11；#555 與 #483 同一類；沒有已合併的修正 | 上游修復，或本機遇到 |
| #567、#569 | 監看 | Windows 無頭 UIA 動作啟動背景視窗（AutoSetFocus）、提權視窗快照空白卻回 `complete: true`；與本 fork 無頭使用最相關，無 PR | 上游修復，或本機遇到 |
| #479、#552 | 監看 | Word 內文未暴露、`launch calc` 逾時等滿；無 PR | 上游修復，或本機遇到 |
| #562 | 不適用 | EnumWindows 在 Codex sandbox 失敗；ADR-007 已記 `window_enum.rs` 的 `attach_input_desktop()`，fork 的 34 檔差異包含它 | Agent 子 shell 又出現 `EnumWindows failed` |
| #510、#537 | 等對應 PR 合併 | #510 對應 #515；#537 對應 #538；AGENTS.md 已規定分平台 cargo | PR 合併 |
| #512 | 不適用 | `scripts/jev` | 見 #517 |
| #491、#492、#494、#529、#533、#534、#548、#550、#554 | 不適用 | macOS／Linux 專屬或跨平台建議；#554（MCP stdio）已關閉 | 改發 macOS／Linux |

- **水位**：`reviewed_through` 設為 `2b0a7334`、`reviewed_release` 為 `v1.2.0`、`reviewed_pr_through` 為 569、`reviewed_issue_through` 為 577（檢查器列出的上限，不得更高，否則 release 之後開的項目被永久隱藏）。`monitored` 在 #456、#400 之外新增 #530、#543、#483、#561、#560 的 head（皆為 2026-11-10 唯讀 `gh pr view` 的值）；#456、#400 仍 open 且 head 未變。
- **移植 #522（open，head `a56aad93`，+1/-1）**：`scripts/check-architecture.sh` 在合併前後的 Windows 都 exit 1（`self-test: G11 did not fire for its planted case in crates/core/src/tree/mod.rs`；`docs/fork/WINDOWS-LIB-TESTS.md` 的「本機門禁範圍」已記同一訊息），`.githooks/pre-commit` 的 Architecture boundaries 因此拒絕提交。讀 #522：自測把 `@ABS@` 換成 Windows 路徑（反斜線）而 G11 以正斜線比對。先紅：改前 `bash scripts/check-architecture.sh` exit 1，輸出同上；後綠：套用 #522 的一行（`tmp` 改 `tmp.replace(os.sep, "/")`）後 exit 0，輸出 `architecture check passed`。這個檔屬上游持有但不在 `check_divergence.py` 的範圍（`crates`、`src`、`apps`、`npm`），所以登記在 `FORK.md` 第 2 節表中；上游合併 #522 後同步取上游版本。
- **合併後閘門**：`cargo fmt --all -- --check`、四套件 `clippy --all-targets -D warnings` 皆 exit 0；`cargo test -p agent-desktop-core --lib` 第一次跑有 1 筆失敗（`trace::trace_read::input_tests::concurrent_session_writers_share_the_remaining_event_budget`，parent 自己的 `emit` 回 `lock_timeout`，與此測試在合併前的檔案就存在）；單獨重跑 3 次各 exit 0、再跑整個 core lib 也 exit 0（1585 筆），判定為負載造成的鎖逾時，不是合併造成；`cargo test --no-fail-fast -p agent-desktop -p agent-desktop-ffi --tests` exit 0，其中只有 README 的 ABI major 一筆因合併失敗，已改文件。
- **pre-commit 的 lib 測試**：從 Git Bash 發起提交時，`.githooks/pre-commit` 的全套 `cargo test --lib` 三次都是 `1479 passed; 3 failed; 6 ignored`，失敗的 3 筆在 `system::launch::tests::live::`（`bare_name_never_chooses_planted_binary_in_current_directory` 的 `image path` panic，另兩筆是它持有 `NOTEPAD_LAUNCH_LOCK` 時 panic 造成的 `PoisonError`）。當時的觀察：`launch_app_impl` 成功回傳 pid，但 8 秒內沒有該 pid 的視窗（`window: None`），查 image path 時 pid 已結束；合併沒動 launch 程式或其測試；合併 commit 改從 PowerShell 發起、hook 完整執行後通過，沒有跳過 hook。**2026-11-12 根因追查（`f94dfa0d` 乾淨 worktree）**：(1) 該測試真正啟動的是作業系統自帶的 `C:\Windows\System32
otepad.exe`，還把測試替探針準備的參數（`--exact tree::fixture::… --ignored --nocapture`）交給它；測試其實不在測 Notepad，卻把「這個外部二進位啟動後存活並出現視窗」當前提。(2) 事故當天的 `notepad.exe` 是 10.0.26100.9278，Application 事件記錄在 2026-11-10 09:10 至 10:35 之間有 13 筆該檔的 APPCRASH（事件 1000，例外 `0xc0000005`，偏移 `0x8332`，版本與偏移完全一致），自 10-25 起的記錄裡沒有任何其他一筆。行程啟動後崩潰，正好對應「pid 已結束、沒有視窗」與連帶的 `PoisonError`。這是時間與簽名的對應，不是重現：9278 版只剩 WinSxS 內一份，從那裡啟動會在約 240 ms 內以代碼 0 結束，與 System32 內的行為不同，無法拿來重跑崩潰。(3) 該檔之後被服務更新（WinSxS 的 9818 於 11-10 19:38 暫存，現為 9949）。在 9949 上，乾淨 worktree 的 Git Bash 單跑該測試 20/20、live 群組完整並行 6 輪（其中 3 輪 24 核心滿載）、經 `scripts/cargo-test-isolated-home.sh` 全套 windows lib（exit 0）與一次真實 `git commit` 觸發的 pre-commit（`1482 passed; 0 failed; 6 ignored`）全數通過，PowerShell 同樣通過。所以「Git Bash 才失敗、PowerShell 才通過」的對照在現況重現不了，上文「環境最小化、cmd.exe 皆無法消除差異」的敘述只能讀作「崩潰是間歇的，它與發起 shell 的對應是巧合還是因果未能判定」，不能當根因。(4) 否證「Notepad 是 Store 轉接 stub」的假設：在 9949 上用產品 `agent-desktop launch notepad.exe --no-attach` 得到 pid 與視窗屬同一個行程（image 為 `System32
otepad.exe`）；只有經 ShellExecute 的啟動（PowerShell `Start-Process`）才會轉給封裝版 Notepad、原 pid 約 0.4 秒後結束，`CreateProcessW` 不會，所以產品的 `create_process` 沒有這個缺陷。(5) 同一個測試還有一個確定性的缺陷：只要維護者本機已開著任何 Notepad，`attach_if_running: false` 就會以 `ActionFailed: Application is already running` 拒絕啟動（修前 5/5 在 `launch_live_tests.rs:298` 失敗，與 shell 無關）。**修法**：該測試不再啟動 Notepad。bare name 的解析檢查（植入同名誘餌後解析結果仍是系統目錄）對 `notepad.exe` 與 `ping.exe` 都保留，因為解析不需要行程；實際啟動改用 `ping.exe -n 30 127.0.0.1`（無視窗、`timeout_ms: 0`），斷言被啟動行程的 image 是解析出的系統 `ping.exe` 而非植入檔。其餘 live 測試改用 `notepad_launch_lock()`，鎖被毒化時仍取得 guard，單一失敗不再連帶出 `PoisonError`。修後 Notepad 已開啟時該測試 3/3 通過，live 群組 9/9 通過。未改產品碼。
- **Windows lib 測試**：ROADMAP 記載的 8 筆失敗在乾淨 `c34b8b9f` 的 `cargo test -p agent-desktop-windows --lib`（HOME、USERPROFILE 指到暫存目錄、未設 `AGENT_DESKTOP_HOME`，BelowNormal）為 `1543 passed; 0 failed; 6 ignored`，沒有重現；與 `docs/fork/WINDOWS-LIB-TESTS.md` 的分類（多為並行負載逾時與桌面競態）一致，不代表那 8 筆已被消除。

## ADR-010: 修 main CI 兩處紅燈：codeql-action 釘點，與 apps/docs 的 minimatch 連同 glob 一起升（2026-11-10）

- **日期**：2026-11-10
- **狀態**：Accepted
- **觸發**：`Dependency freshness` workflow 失敗（`tools/check_dependency_freshness.py --strict` 報 `github/codeql-action/init`、`analyze` 宣告 4.38.2、最新 4.39.3）；Dependabot 告警 #5（`minimatch`，medium，`apps/docs/pnpm-lock.yaml`，受影響 `>= 7.0.0, < 11.0.1`，修補版 11.0.1），Dependabot 自動更新失敗（run 42744296217）。
- **決策一：repin codeql-action 到 v4.39.3**。`gh api repos/github/codeql-action/git/ref/tags/v4.39.3` 回 annotated tag（`f3bbb056`），再解 `git/tags/f3bbb056` 得 commit `33d7b85da1a04171e211befe2c739303b9a4c066`。`.github/workflows/codeql.yml` 的 init、analyze 共 4 行改到此 SHA，註解 `# v4.39.3`。原 ADR-006 第 3 點的 `dependency-deferrals.json` 兩筆 codeql 延後（4.39.1）因此失去對象，已刪除。這 4 行不再與 upstream/main 相同；上游 workflow 檔下次同步若衝突，取較新的 SHA。
- **決策二：apps/docs 的 minimatch 告警（GHSA-g3hv-75wg-m249，Dependabot #5）修，方法是 minimatch 與 glob 一起升**（取代同日稍早的「不修、登記延後」）。`apps/docs` 是上游帶進來的文件站，fork 不發佈，CI 不建置它；minimatch 是間接依賴（`@ts-morph/common@0.28.1` 與 `glob@11.1.0`，後者由 `@vercel/geistdocs@2.14.0` 使用），唯一修補版是 11.0.1 起。
  - **第一次嘗試（commit `5cd2c10e`，已由 `c1d86932` 撤回）**：只用 pnpm override 把 minimatch 升到 11.0.1。審查實測發現 `glob@11.1.0` 用 `instanceof RegExp` 判斷比對元素，minimatch 11 改成 Matcher 物件後，`**/*.md`、`**/*` 等萬用字元一律靜默回 `[]`，CI 不建置 apps/docs，不會變紅。所以只升 minimatch 不可行。
  - **為什麼先前結論「沒有升級路徑」不完整**（2026-11-10 `npm view`）：`@vercel/geistdocs` 最新 2.14.4 仍宣告 `glob ^11.0.0`、`@ts-morph/common` 最新 0.29.0 仍宣告 `minimatch ^10.0.1`，這兩點屬實；但 glob 自己早已有宣告 minimatch 11 的版本：`glob@14.0.0` 起 `minimatch ^11.0.0`，`14.0.4`（2026-11-06）宣告 `minimatch ^11.0.5`、`path-scurry ^3.0.0`、`minipass ^7.1.3`。所以不用等 geistdocs，用 override 讓 geistdocs 底下的 glob 換成 14.0.4 即可。
  - **相容性評估（讀 `@vercel/geistdocs@2.14.0` 發佈包 `dist/cli/index.js`）**：geistdocs 只在兩處呼叫 glob，都是 `import { glob } from "glob"` 的非同步函式：`translate` 的 `glob(pattern, { cwd, absolute: true, nodir: true })`（預設樣式 `content/docs/**/*.mdx`），與 `update` 的 `glob(patterns, { cwd, nodir: true, dot: true, ignore: [".git/**", ...] })`（預設 `**/*`）。這四個選項在 glob 14 的 README 仍在。11 到 14 的破壞性變更：glob CLI 搬到 `glob-bin`（13 起；geistdocs 不用 bin）、engines 由 `20 || >=22` 放寬為 `18 || 20 || >=22`（本 repo node v26 符合）。**更正**：先前寫的「`geistdocs search sync` 會產出空索引」機制有誤，`search sync` 讀的是 Next 建置後 `.next` 的預渲染輸出（`prerender-manifest.json` 與 `export.body`），不經 glob；受影響的是 `translate`、`update` 兩個指令（找不到檔案）。geistdocs 其餘地方不用 glob；`ts-morph` 在 geistdocs 只以記憶體檔案系統 `createSourceFile` 解析 `geistdocs.tsx`，沒有走 `@ts-morph/common` 的 `matchGlobs`。
  - **修法**：`apps/docs/pnpm-workspace.yaml` 的 `overrides` 加 `"@vercel/geistdocs>glob": 14.0.4` 與 `minimatch: 11.0.5`（11.0.5 於 2026-11-05 發佈，`minimumReleaseAge: 2880` 即兩天，pnpm 解析通過）。`@ts-morph/common@0.28.1` 宣告的是 `minimatch ^10.0.1`，override 是硬把它換到 11；它只在 `NodeRuntime.getPathMatchesPattern` 與 `BrowserRuntime.getPathMatchesPattern` 呼叫 `minimatch.minimatch(path, pattern)`。lockfile 用 pnpm 11.1.3 `--lockfile-only` 重產，diff 只有：移除 `glob@11.1.0`、`minimatch@10.2.6`、`path-scurry@2.0.2` 與僅由 glob 11 帶入的 `foreground-child`、`jackspeak`、`@isaacs/cliui`、`package-json-from-dist`、`signal-exit`；新增 `glob@14.0.4`、`minimatch@11.0.5`、`path-scurry@3.0.0`、`@isaacs/nonrecursive-glob-match@1.1.0`（minimatch 11.0.5 的新依賴）。lockfile 中 minimatch 只剩 11.0.5，`pnpm why minimatch glob` 顯示 glob 只有一份 14.0.4（僅 geistdocs 使用），沒有其他 glob 消費者。
  - **驗證（在空的暫存 HOME/USERPROFILE 下，升級前後兩棵樹各自 `pnpm install --frozen-lockfile --ignore-scripts`，逐項比對）**：(a) 用 geistdocs 實際的兩種呼叫形狀對真實的 `apps/docs`（38 個 mdx）跑 11 個樣式（`content/docs/**/*.mdx`、`**/*.{md,mdx}`、`**/*`＋`ignore`＋`dot`、`{a,b}`、`[a-c]`、`@(…)` 等），升級前後結果逐項完全相同，其中 5 項與不經 glob 的 fs 走訪基準逐項一致，其餘幾項的差異只是沒設 `nodir` 時多回傳名稱像 `sitemap.md`、`llms.mdx` 的路由目錄；合成樹重現腳本在 glob 14＋minimatch 11.0.5 下的 `**/*.md`、`**/*.{md,mdx}`、`**/*`、ignore 皆非空且與舊版一致（`Minimatch.set` 元素型別為 `Matcher`，證明確實跑在 minimatch 11 的資料結構上）。(b) `minimatch(path, pattern)` 對 26 個路徑 × 43 個樣式（絕對路徑、Windows 磁碟機路徑、`!(…)`、`{1..3}`、dot 目錄、`node_modules`）共 1118 組，10.2.6 與 11.0.5 零差異。(c) 兩棵樹 `fumadocs-mdx` 與 `next build` 皆 exit 0、輸出相同，預渲染檔案清單各 657 個且名稱雜湊相同，`/desktop/api/search/export` 的 `export.body` 位元組相同（38 份文件）。(d) 真的跑 `geistdocs translate`（無參數與不存在的樣式），升級前後都回報 `Found 38 MDX file(s)` 與 `No MDX files found matching pattern`。
  - **驗證的限制**：`geistdocs search sync` 在升級前後的樹上都停在 `/api/search/export was not statically generated by the latest Next.js build.`（CLI 寫死該路徑，而本站的路由在 microfrontends 前綴下是 `/desktop/api/search/export`），與本次變更無關，所以沒能驗證上傳那一段；它不經 glob，上述 `export.body` 相同已涵蓋其輸入。`translate` 驗證時 CLI 越過檔案搜尋後開始翻譯，向 CLI 預設的翻譯服務發了請求（內容是上游公開文件的 mdx，只發生在暫存副本）。Dependabot 告警是否在 push 後自動關閉尚未驗證。
  - **登記方式**：這兩條 override 登記在 `docs/DIVERGENCE.md`。`.github/dependency-deferrals.json` 只處理 `check_dependency_freshness.py` 讀的宣告（Cargo、Actions），不讀 apps/docs，所以不放進去。**不在 GitHub 上 dismiss 告警**，那由維護者決定。
  - **移除條件**：`@vercel/geistdocs` 自行依賴 glob >= 14（或相容 minimatch 11 的 glob）且 `@ts-morph/common` 宣告 minimatch >= 11.0.1 後，兩條 override 都可刪；兩者缺一不可單獨留 `minimatch` override，否則重現 `5cd2c10e` 的靜默空結果。

## ADR-011: 獨立覆核 v1.2.0 後的加固：MCP 預設關閉危險工具、launch 拒絕 UNC、暫定落點型別化（2026-11-11）

- **日期**：2026-11-11
- **狀態**：Accepted
- **觸發**：獨立覆核（ocx-gpt-6-luna，唯讀靜態讀碼）對合併進來的上游 v1.2.0 提出 7 項；每項先由本 fork 自行讀碼確認再修。
- **F1（高，上游 #519 引入）**：MCP 把所有葉指令公開成工具，`press`、`key-down`、`key-up` 的 `--force` 出現在 schema，可繞過 Windows 危險組合封鎖（`win+r` 等）而執行任意指令；`launch`、`batch`、會寫任意路徑的 `screenshot PATH`、`trace export --out`、`clipboard-get --out`、`session gc` 也都可達。決定：MCP 沒有既有使用者，所以直接改預設。`launch`、`batch`、檔案輸出（連同 `session gc`）、DevTools 與 inspector 參數預設關閉，分別以 `agent-desktop mcp --allow-launch`、`--allow-batch`、`--allow-file-write`、`--allow-cdp` 開啟；`force` 與 `launch cwd` 永遠不提供。檔案路徑限制在 `<session 目錄>/mcp-output`（不是整個 session 目錄，否則可覆寫 manifest 與 trace），開啟時必須有 `--session` 或 `AGENT_DESKTOP_SESSION`。限制分兩層：目錄過濾 schema，另外伺服器把限制放進子行程環境變數，子行程在 `command_policy::preflight` 對已解析的 `Commands` 強制，所以 batch 項目（同一個 preflight）無法繞過；只過濾 JSON 會漏掉 batch 內的 `force`。文件寫明 MCP client 等同使用者，不是沙盒。
- **F2（中，既有）**：`launch_path.rs` 接受 UNC，`resolve_executable` 的 `is_file` 對 `\\server\...` 發 SMB（NetNTLM 洩漏）並可執行遠端 exe。決定：所有模式（CLI 與 MCP）拒絕 UNC、`\\?\UNC\`、`\\.\` 裝置路徑與正斜線寫法，只認 `X:\` 與 `\\?\X:\`，在碰檔案系統之前回 `INVALID_ARGS`；`launch --cwd` 也套同一規則。
- **F3（中低，既有）**：`launch --cwd` 任意目錄可致 DLL 劫持。決定：MCP 移除 `cwd`；CLI 保留，文件警示只傳可信目錄。
- **F4（低）**：`--arg=--remote-debugging-port=...` 可繞過 `--cdp`。決定：沒有 `--allow-cdp` 時，`--cdp` 與以 `remote-debugging`、`remote-allow-origins`、`inspect` 開頭的 `--arg`（忽略大小寫，`-`、`--`、`/` 前綴皆算）一律拒絕，batch 項目同樣適用。
- **F5（低，上游 #442）**：`resolve_errors.rs` 與 `resolve_saved_ref.rs` 的錯誤建議引導 agent 改用 `--headed` 座標點擊挑選相同控制項，等於按位置猜。決定：改為「重新 snapshot 挑唯一控制項，或請人確認」，skills 與文件同步。
- **F6（低）**：`land_saved_ref` 的暫定 handle 只靠呼叫端自律不拿去 dispatch。決定：新增 `ProvisionalHandle`（無 Deref、無轉換，唯一出口 `for_reads`），`land_within_deadline` 回 `ResolveAttemptOutcome<ProvisionalHandle>`。dispatch 拿的是完整 `resolve_saved_ref` 的 `NativeHandle`，編譯器擋下把暫定 handle 放進 dispatch 位置。限度：`for_reads` 回 `&NativeHandle`，且公開的 `execute_verified_action` 簽章（FFI 也用）未改，所以保證是「不經明確的 `for_reads` 就過不了編譯」，不是不可偽造。
- **F7（低）**：`recorded_window` 在 `source_window_id` 缺席時取同 pid 第一個視窗。決定：同 pid 且同 generation 的候選多於一個就回 `AMBIGUOUS_TARGET`（`details.kind: saved_ref_window_ambiguous`）；函式因 `resolve_saved_ref.rs` 已 399 行而拆到 `recorded_window.rs`。
- **未處理**：`close-app --force`（終止程序）仍在 MCP 公開，它不執行程式也不繞過封鎖清單，列在 ROADMAP；其他指令的 UNC 檔案路徑參數（CLI 的 `clipboard-set --image` 等）在 CLI 模式未拒絕，MCP 模式由輸出目錄限制擋下。
