# 路線圖

> 本 repo 後續工作的單一真相源。做完一項，就在同一個 commit 刪掉那一列；新工作只加在這裡。
> 現況段的數字是 2026-11-02 依 `main` `a67bf683` 查的快照，開工前先重查（指令見各列「證據」）。

## 現況
`vercel-labs/agent-desktop`（Rust，OS 無障礙樹桌面操控 CLI 與 C 函式庫）的個人 fork，維護重心是 Windows 11 原生（UI Automation）。2026-11-02 查：本 fork 是 `upstream/main` `9eb9b296`（`Video remove (#421)`）加 7 個 fork 自己的 commit，`git rev-list --left-right --count HEAD...upstream/main` 回 `7 0`，所以 commit 軸沒有待審項目，與 `FORK.md` 第 1 節「完整追蹤 `upstream/main`」一致。上游 PR 與 issue 從未審過，水位在 `tools/upstream_baseline.json` 刻意設為 0：`python tools/check_upstream_updates.py` 列出 PR 381 筆（上游 `gh pr list --state all`：已合併 239、關閉未合併 81、開著 61）與 issue 81 筆（開著 41、已關 40），合計 462 筆待審。注意上游把堆疊 PR 合進非 `main` 的分支（#445、#447 合進 `laf/identity-linux-windows`），所以「已合併」不等於已在本 fork。`main` 最近一次 push（`a67bf683`）的 CI、CodeQL、Fork Maintenance、Supply Chain 皆 success；code scanning 3 筆告警全為 fixed、Dependabot 告警 0、分支保護未設（`gh api repos/SanHsien/agent-desktop/branches/main/protection` 回 404）、fork 的 issue 功能關閉、已有 GitHub release `v1.0.2`（`gh release list -R SanHsien/agent-desktop`）。Windows 本機閘門 2026-11-02：`cargo fmt`、`clippy -D warnings`、binary 與 FFI 測試綠；`agent-desktop-windows --lib` 在維護者本機 8 筆失敗，其中 5 筆單獨重跑通過（負載造成的逾時），另外 3 筆見近期表。

## 近期：可以直接動手
| 項目 | 為什麼 | 驗收方式 | 建議執行者 | 證據 |
| --- | --- | --- | --- | --- |
| 審上游 PR 軸 381 筆：先審開著的 61 筆與標題含 `windows`／`linux` 的，再往低號走；每批把結論與證據寫進 `docs/fork/DECISIONS.md`，再提高 `reviewed_pr_through` | 從未審過，被關掉的 81 筆正是 PLAYBOOK §2 判準一說的「上游拒收但可能有價值」；本 fork 的 Windows 加固與這些修正可能重疊（例：#400、#456、#457 為開著的 `windows` PR，#440、#366、#367 為關閉未合併的 `windows` PR） | `python tools/check_upstream_updates.py --strict` 的 PR 區段只剩尚未審的號碼；每個結論有 diff 檔案與行數、本 fork 對應檔案實況、觸發條件（PLAYBOOK §2 判準二）；PR 目前 head 已掃隱藏內容（判準八） | Claude Sonnet | 檢查器輸出前 10 筆為 #1 至 #10（`chore(main): release 0.1.x` 與早期 feat）；`tools/upstream_baseline.json`；`gh pr list --repo vercel-labs/agent-desktop --state all --limit 1000`（唯讀） |
| 審上游 issue 軸 81 筆，從 Windows 相關與 2026-10 後新開的開始 | 同上；#459「Windows: headless type refuses SetValue-only edit controls」之類可能是本 fork 自己會踩到的缺陷 | `reviewed_issue_through` 提高並附結論；確認缺陷在本 fork 仍存在再決定移植（PLAYBOOK §2 判準四） | Claude Sonnet | 檢查器輸出前 10 筆為 #18、#21、#23、#34、#39、#49、#84、#94、#95、#141；`gh issue list --repo vercel-labs/agent-desktop --state all --limit 1000`（唯讀） |
| 查 8 筆 Windows lib 測試在維護者本機失敗的原因，尤其 `a_launched_process_never_inherits_the_lease_handoff_key`（單獨重跑仍失敗） | 同一份程式在 GitHub `Test (Windows)` 是綠的，差別在本機環境；懷疑 `cmd.exe` 的環境傾印用本機 ANSI 碼頁、測試以 UTF-8 讀取，尚未驗證。另外 5 筆（Action Center 開啟逾時等）單獨重跑通過，疑為負載造成 | 在乾淨環境（不含非 ASCII 的環境變數）跑 `scripts/cargo-test-isolated-home.sh test -p agent-desktop-windows --lib` 全綠；若確為測試對語系的假設，修成與語系無關並加測試 | Claude Sonnet | `crates/windows/src/system/launch_lease_isolation_tests.rs:88`（`read_to_string` 回 `stream did not contain valid UTF-8`）；`gh run list -R SanHsien/agent-desktop -w CI` 的 `a67bf683` 為 success |
| 在維護者本機真機跑一次 Windows live E2E 並留下結果 | CI 只跑 harness 的自我測試，沒有任何 workflow 在真實桌面上驅動 `agent-desktop.exe`；`tests/e2e-windows/README.md` 明說 U6 自我測試「cannot run on a hosted runner」；`native-e2e.yml` 是 macOS 自架 runner。Windows 加固（桌面隔離、剪貼簿競態）的實機效果只有人跑過才算數 | `powershell -NoProfile -File tests\e2e-windows\Run-E2E.ps1`（先 `cargo build --release -p agent-desktop`）全部 leg 通過或有 `skip-allowlist.psd1` 內的理由；結果摘要記進 `docs/fork/DECISIONS.md` | 維護者 | `.github/workflows/ci.yml:477-498`（只有 self-test）；`.github/workflows/native-e2e.yml:15`（`[self-hosted, macOS, agent-desktop-e2e]`）；`.github/workflows/windows-capability-probe.yml`（只在 probe 路徑的 PR 與手動觸發） |
| 決定要不要設分支保護 | `main` 目前沒有保護，推送與強推都不受擋 | 維護者決定後：`python -m fleet_ops protect agent-desktop --context "<真的回報過的檢查名稱>"`；必要檢查只取 `Test (Windows)`、`Format` 這類最近成功 run 的 job 名稱；依 PLAYBOOK §3 不設目前會紅的 job | 維護者 | `gh api repos/SanHsien/agent-desktop/branches/main/protection` 回 404；`gh run view` 的 job 名稱：`Format`、`Test (Windows)`、`Rust 1.89 MSRV` 等皆 success |
| 修正 `docs/fork/DECISIONS.md` ADR-002 第 3 點：它寫「實作 `tools/verify_a11y_tree_snapshot.py`」，但這個檔案不在本 repo | 文件宣稱的驗證工具查不到；規格書說它在 `ai-inbox` | 改成指向實際位置或刪掉該點；`tests/fork` 通過 | Claude Haiku | `docs/fork/DECISIONS.md:29`；`docs/agent-desktop-computer-use-spec.md:224`（寫 `ai-inbox/tools/verify_a11y_tree_snapshot.py`）；`git ls-files` 的輸出中沒有任何 `verify_a11y` 檔名 |

## 中期：等條件成立再做
| 項目 | 觸發條件 | 驗收方式 | 建議執行者 | 證據 |
| --- | --- | --- | --- | --- |
| 處理依賴新鮮度的真實紅燈：Cargo `futures-util =0.3.33`（最新 0.3.34）、`rustix =1.1.5`（最新 1.1.6），以及 Actions `download-artifact`、`setup-node`、`setup-python`（ci.yml 的 v7.0.0）、`upload-artifact`、`codeql-action` | 上游 `main` 升了這些釘版（Cargo `futures-util`、`rustix`、Actions），或 deferral 因出現更新版而失效；這些檔案歸上游（`crates/linux`、上游 workflow），在本 fork 改或合併動到它們的 Dependabot PR，日後同步都會衝突 | 經 commit 軸同步上游後 `python tools/check_dependency_freshness.py --strict` exit 0，或只剩寫了理由的 deferral | Claude Sonnet | 2026-11-02 報告：Cargo 2 筆與 Actions 6 筆 `REVIEW UPDATE`；`crates/linux/Cargo.toml:16,18`；`.github/dependabot.yml`；當天 `gh pr list -R SanHsien/agent-desktop --state all` 為空，尚未看到 Dependabot 開過 PR |
| 同步上游堆疊分支落到 `main` 的內容（例：`laf/identity-linux-windows` 的 #445、#447） | 上游把那些分支合進 `main`，commit 軸出現新 commit | `python tools/check_upstream_updates.py` 列出新 commit；逐筆審後移植或合併，水位推進，閘門全綠 | Claude Sonnet | `gh pr list --repo vercel-labs/agent-desktop --state merged`：#445、#447 的 `baseRefName` 是 `laf/identity-linux-windows` |
| 跟上游升級 `zbus` 到 6 並移除 `deny.toml` 的三筆 RUSTSEC 例外 | 上游 `crates/linux` 改用 zbus 6（`git log upstream/main -- crates/linux/Cargo.toml`） | `crates/linux/Cargo.toml` 的 zbus 不再是 `=5.19.0`；`deny.toml` 的 `RUSTSEC-2026-0370` 至 `0372` 刪除；`.github/dependency-deferrals.json` 的 `zbus` 條目刪除；`cargo deny check` 通過 | Claude Sonnet | `deny.toml:5-7`（理由原文：zbus 5.19 是有 blocking API 的最新版）；`.github/dependency-deferrals.json`（deferral 在 crates.io 超過 6.2.1 時自動失效）；該理由取自 `deny.toml`，未對照 zbus 6 changelog 驗證 |
| 上游發 1.1.0 時審 release 並決定是否跟進 | 上游 `chore/release-1.1.0` 分支合進 `main` 或出現 `v1.1.0` tag | 審 CHANGELOG 與產品檔差異；本 fork 的版本號與 tag 政策（目前 release `v1.0.2`）寫進 `docs/fork/DECISIONS.md` | Claude Opus | `git ls-remote --heads upstream`（2026-11-02 共 177 個分支，含 `chore/release-1.1.0`） |
| 把上游 PR 軸的 Windows 修正回貢上游 | 維護者在**當次對話**明確同意回貢 | PR 開在 `vercel-labs/agent-desktop` 前先讀 `FORK.md` 第 1 節；回貢標記 `# upstream-ok:` | 維護者 | `FORK.md` 第 1 節「回貢判準」 |

## 刻意不做
| 不做什麼 | 理由 | 什麼情況下重新考慮 |
| --- | --- | --- |
| 把 `reviewed_pr_through`／`reviewed_issue_through` 直接設成上游目前最大號 | 沒有人審過，設了就是假稱已審；堆疊 PR 還讓「已合併」不等於已在本 fork | 不重新考慮，除非像 buzz 一樣在 `docs/fork/DECISIONS.md` 明記「水位以前不分流」並附理由（buzz commit `69e5837`）；要降噪就分批審完再提高 |
| 開上游分支軸（`check_branches`） | 上游有 177 個分支，多數是功能與堆疊分支，報告會被淹沒；落到 `main` 的內容會經 commit 軸抵達 | 上游開始用長期維護的 release 分支（例：security backport） |
| 在本 fork 內直接改 `crates/linux/Cargo.toml` 的釘版或上游 workflow 的 action 版本來讓新鮮度變綠 | 那些檔案歸上游，改了日後上游同步全是衝突；升級靠 Dependabot 與上游同步 | 上游停止維護這些檔案 |
| 讓新鮮度檢查讀 `apps/docs`（上游的文件站）與 `npm/package.json` | 文件站本 fork 不建置也不部署，Dependabot 也不掃；`npm/package.json` 沒有任何依賴，讀它是裝飾 | 本 fork 開始自己部署文件站，或 `npm/package.json` 出現依賴 |
| 對上游做任何 push、PR、issue | `FORK.md` 第 3 節：`upstream` 只追蹤不推送 | 維護者在當次對話明確說要回貢 |
| 壓縮歷史 | ADR-001 決定保留完整歷史以便跟進上游 | 不重新考慮 |

## 維護方式
個人 fork，在 `main` 上做。依賴由 Dependabot（cargo、npm、github-actions）與每月 `Dependency freshness` 追蹤；上游由每週 `Upstream check` 追蹤，檢查器是艦隊的標準版（正本在 `repo-fleet-ops` 的 `tools/canonical/check_upstream_updates.py`），不在這裡改程式、只改 `tools/upstream_baseline.json`。合併前 `pwsh -NoProfile -File tools/dev_check.ps1 -Full`，Rust 部分在 Windows 以 `cargo fmt --all -- --check`、`cargo clippy -p agent-desktop-core -p agent-desktop-windows -p agent-desktop -p agent-desktop-ffi --all-targets -- -D warnings`、`scripts/cargo-test-isolated-home.sh test ...`（`--lib`、`-p agent-desktop`、`-p agent-desktop-ffi --tests`）驗證。上游持有的檔案（`crates/`、`src/`、`apps/`、`npm/`）原樣保留。
