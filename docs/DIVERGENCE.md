# 產品檔案與上游的差異

Base: `2b0a7334fc111730454b726e6800aa41cdafc093`

基準為上游 v1.2.0 release commit `chore: release 1.2.0 with mcp support (#568)`（2026-11-10 自 `9eb9b296` 的 `Video remove (#421)` 移來，逐檔差異集合不變，見 ADR-009）。本表只列 `crates/`、`src/`、`apps/`、`npm/` 的差異；fork 維護文件、工具與 workflow 見 [FORK.md](../FORK.md)。來源為 `8d372752` 的 Windows 修正、`0f7e8e65` 的格式整理及 `a67bf683` 的測試拆檔。

## 檢查與移除

```powershell
python tools/check_divergence.py
python tools/check_divergence.py --target HEAD
git diff --name-only --no-renames "$(git merge-base upstream/main HEAD)" HEAD -- crates src apps npm
```

預設檢查工作樹，包含產品目錄內未被忽略且尚未追蹤的檔案；`--target HEAD` 只檢查 commit。檢查器預設一律以上方釘住的 Base 為比較基準，不連網、不讀 `upstream/main`，所以有沒有 upstream remote 結果都相同；完整歷史必須包含該 commit。上游合併進來後舊 Base 會把上游檔案列為 `unlisted`，須同步更新 Base 與清單。可用 `--base <ref>` 明確指定比較基準（手動比對上游時用 `git merge-base upstream/main HEAD`，不要對移動中的 `upstream/main` 做兩點比較），無法解析時不退回其他基準。

```text
Git 產品差異清單 + 表格
          |
          +-- 路徑相同、每列有理由及移除條件 -> exit 0
          +-- 漏列或多列                     -> exit 1
          +-- 格式錯誤、Git 或讀檔失敗       -> exit 2
```

`--no-renames` 讓重新命名的舊、新路徑都接受審查。上游 ref 改變後，先逐檔重審，再更新 Base 與表格。上游已有等效行為且相應 Windows 測試通過，才能移除功能差異；測試放寬、特權不足跳過與忽略清理錯誤並非產品修正，移除條件另外列明。格式或拆檔差異可以在保留測試覆蓋與 source rules 通過時取回上游內容。

## 逐檔清單

| Path | Reason | Removal condition |
| --- | --- | --- |
| `apps/docs/content/docs/commands/app-window.mdx` | launch 的 `APP` 與 `--cwd` 說明加上：Windows 拒絕網路與裝置路徑、`--cwd` 只傳可信目錄（它決定新程序先載入哪份 DLL）、MCP 用戶端不能設 `cwd`，見 ADR-011 | 上游文件涵蓋同樣的警示，或本 fork 取回上游版本 |
| `apps/docs/content/docs/guides/mcp.mdx` | Security 段改寫：MCP 用戶端等同使用者而非沙盒，列出 `--allow-launch`、`--allow-batch`、`--allow-file-write`、`--allow-cdp` 與永遠生效的限制（`force` 不提供、檔案路徑限制在 session 輸出目錄），見 ADR-011 | 上游 MCP 伺服器自帶等效預設關閉的限制與文件，或本 fork 取回上游版本 |
| `apps/docs/content/docs/snapshots-and-refs.mdx` | AMBIGUOUS_TARGET 列的建議由「座標點擊可選其一」改為「重新 snapshot 挑唯一控制項，或請人確認」，與錯誤訊息一致，見 ADR-011 | 上游文件與錯誤建議都不再引導 agent 用座標點擊挑選相同控制項 |
| `apps/docs/pnpm-lock.yaml` | minimatch 10.2.6 升到 11.0.5、`@vercel/geistdocs` 底下的 glob 11.1.0 升到 14.0.4，以修 Dependabot 告警 #5（minimatch < 11.0.1 受影響），見 ADR-010 | 上游 `apps/docs` 的 lockfile 已解到 minimatch >= 11.0.1，且 glob 版本宣告 minimatch ^11 |
| `apps/docs/pnpm-workspace.yaml` | `overrides` 加 `@vercel/geistdocs>glob: 14.0.4` 與 `minimatch: 11.0.5`（兩者必須一起，只升 minimatch 會讓 glob 11 的萬用字元靜默回空），見 ADR-010 | `@vercel/geistdocs` 自行依賴 glob >= 14 且 `@ts-morph/common` 宣告 minimatch >= 11.0.1，兩條 override 可刪 |
| `crates/core/src/ref_action/presentation.rs` | 游標疊加的目標中心經 `ProvisionalHandle::for_reads` 讀 bounds（暫定落點 handle 不再是裸 `NativeHandle`） | 上游以型別區分暫定落點與完整解析結果 |
| `crates/core/src/ref_action/ref_action_poll.rs` | 輪詢迴圈拿到的暫定落點 handle 只經 `for_reads` 交給 actionability 預檢，不能傳進 dispatch | 上游以型別區分暫定落點與完整解析結果 |
| `crates/core/src/refs/mod.rs` | 註冊 `recorded_window` 模組（從 `resolve_saved_ref.rs` 拆出） | 上游有等效模組佈局，或取回 `resolve_saved_ref.rs` 原貌 |
| `crates/core/src/refs/recorded_window.rs` | 新檔：`recorded_window` 與其輔助函式，從 `resolve_saved_ref.rs` 拆出（該檔已 399 行）；entry 沒有 window id 時，同 pid 與同 generation 有多個視窗就回 `AMBIGUOUS_TARGET`（`saved_ref_window_ambiguous`），不再取第一個 | 上游在無 window id 且多個候選視窗時同樣拒絕 |
| `crates/core/src/refs/ref_resolve_deadline.rs` | `land_within_deadline` 回 `ResolveAttemptOutcome<ProvisionalHandle>`，嘗試與分類函式改為泛型；測試指定 `NativeHandle` | 上游以型別區分暫定落點與完整解析結果 |
| `crates/core/src/refs/ref_root_read.rs` | `recorded_window` 的匯入路徑改到新模組 | 隨 `recorded_window.rs` 取回上游而移除 |
| `crates/core/src/refs/resolve_attempt_outcome.rs` | `ResolveAttemptOutcome` 改為泛型（預設 `NativeHandle`），讓暫定落點用 `ProvisionalHandle` | 上游以型別區分暫定落點與完整解析結果 |
| `crates/core/src/refs/resolve_errors.rs` | `identity_not_unique` 的建議不再指向 `--headed` 座標點擊，改為重新 snapshot 挑唯一控制項或請人確認（座標點擊會點到該位置上的任何東西） | 上游建議文字同樣不引導座標點擊 |
| `crates/core/src/refs/resolve_errors_tests.rs` | 新增建議文字不含座標點擊的測試 | 上游涵蓋同樣斷言 |
| `crates/core/src/refs/resolve_saved_ref.rs` | 新增 `ProvisionalHandle`（無 Deref、無轉換；只能 `for_reads`），`land_saved_ref` 回它；`ambiguous_live_match` 建議改為重新 snapshot 或請人確認；`recorded_window` 與 `adapter_error` 移到 `recorded_window.rs` | 上游以型別保證暫定落點不進 dispatch |
| `crates/core/src/refs/resolve_saved_ref_tests.rs` | 落點測試改用 `ProvisionalHandle`；新增 `ambiguous_live_match` 建議文字測試 | 隨 `resolve_saved_ref.rs` 取回上游而移除 |
| `crates/core/src/refs/resolve_saved_ref_window_tests.rs` | 新增無 window id 且兩個同 generation 視窗時拒絕為 `AMBIGUOUS_TARGET` 的測試 | 上游涵蓋同樣斷言 |
| `crates/core/src/session/tree.rs` | 非 Unix session 根目錄也經 `ensure_private`，避免只保護 trace 子目錄 | 上游在建立 session 時同樣保護根目錄，Windows session 測試通過 |
| `crates/ffi/src/actions/result.rs` | 釋放 action result 時用 `as_mut` 檢查空指標；單元測試移到相鄰檔案以符合 400 行限制 | 上游涵蓋空 result 與 post-state 釋放，且檔案大小及 FFI 測試通過 |
| `crates/ffi/src/actions/result_tests.rs` | 從 `result.rs` 拆出的 action result 轉換及釋放測試，包含空 result 測試 | 上游以符合 400 行限制的佈局保留相同測試覆蓋 |
| `crates/ffi/tests/common/mod.rs` | Windows FFI 隔離目錄移除繼承 ACL，授權 SYSTEM、Administrators 與 Owner Rights | 上游 fixture 能在 Windows 原生環境建立被私有檔案檢查接受的隔離目錄 |
| `crates/ffi/tests/common/win32_fixture.rs` | FFI Win32 fixture 執行緒先附掛輸入桌面 | 上游 fixture 在隔離桌面環境可建立及觀測視窗，FFI 實機測試通過 |
| `crates/linux/Cargo.toml` | `rustix` 釘選由 `=1.1.5` 改為 `=1.1.6`，修 RUSTSEC-2026-0486（xattr 函式在零長度緩衝區把未寫入位元組標成已初始化），讓 Supply Chain 的 cargo-deny advisories 通過 | 上游釘選 `rustix` ≥1.1.6 |
| `crates/windows/Cargo.toml` | 啟用 `Win32_System_StationsAndDesktops` 供輸入桌面附掛使用 | 上游提供等效桌面處理，或附掛呼叫全部移除後 Windows 建置通過 |
| `crates/windows/src/input/clipboard/clipboard_bytes.rs` | UTF-16 解碼改用固定大小 `as_chunks`，對齊 clippy 要求 | 上游解碼保留尾端不完整位元組的行為，且 clippy 與剪貼簿測試通過 |
| `crates/windows/src/input/clipboard/clipboard_files_tests.rs` | HDROP 測試的 UTF-16 解碼同步改用 `as_chunks` | 上游測試具有相同對齊與 round-trip 覆蓋，clippy 通過 |
| `crates/windows/src/lib.rs` | 匯出 `attach_input_desktop` 給跨 crate 的 fixture 使用 | 上游有等效公開入口，或跨 crate 的附掛呼叫不再需要 |
| `crates/windows/src/system/capture/capture_routing_parity_tests.rs` | 新增 HomeIsolation 於外部存在 AGENT_DESKTOP_HOME 時維持目錄隔離之單元測試 | 上游具備等效之 AGENT_DESKTOP_HOME 測試隔離回歸覆蓋，且 capture routing 測試通過 |
| `crates/windows/src/system/capture/capture_test_support.rs` | capture 測試的隔離目錄明設私有 ACL，避免環境繼承權限影響測試 | 上游隔離目錄能通過 Windows 私有目錄驗證與 capture 測試 |
| `crates/windows/src/system/capture/capture_window.rs` | 空影格檢查改用固定大小像素區塊，對齊 clippy 要求 | 上游保留空影格判定語意，capture 測試與 clippy 通過 |
| `crates/windows/src/system/capture/gdi_surface.rs` | GDI 建立螢幕 DC 前先附掛輸入桌面 | 上游有等效桌面處理，隔離環境 GDI capture 測試通過 |
| `crates/windows/src/system/cursor_overlay/text_tests.rs` | 用 `fill` 填入卡片像素，取代逐像素賦值以符合 clippy | 上游測試保持相同像素輸入且 clippy 通過 |
| `crates/windows/src/system/interaction_lease/directory_tests.rs` | 缺少 SeRestorePrivilege 時跳過異主目錄測試，並忽略部分清理錯誤；這會減少該環境的測試覆蓋 | 上游 fixture 可在無特權環境驗證拒絕行為，或保留可稽核的跳過理由及清理方式 |
| `crates/windows/src/system/interaction_lease/tests.rs` | lease scratch 目錄明設私有 ACL，避免繼承權限干擾 | 上游 scratch 目錄在 Windows 原生環境通過 lease 私有目錄測試 |
| `crates/windows/src/system/launch.rs` | `absolute_cwd` 同樣拒絕網路與裝置命名空間路徑（工作目錄在 DLL 搜尋順序前段，且指向 UNC 會對遠端發 SMB）；新增 `launch_cwd_tests` 模組宣告 | 上游 `absolute_cwd` 同樣拒絕非本機磁碟路徑 |
| `crates/windows/src/system/launch_cwd_tests.rs` | 新增 `absolute_cwd` 拒絕網路與裝置路徑、保留本機 verbatim 路徑的測試（`launch_tests.rs` 已 392 行，放不下） | 上游測試以符合 400 行限制的佈局涵蓋相同斷言 |
| `crates/windows/src/system/launch_lease_isolation_tests.rs` | 子程序環境傾印明確使用 cmd /U 與嚴格 UTF-16LE 解碼，並驗證非 ASCII 值，避免將本機碼頁誤當 UTF-8 | 上游具備與碼頁無關的子程序 lease key 隔離測試，且在碼頁 950 與 65001 通過 |
| `crates/windows/src/system/launch_live_tests.rs` | `bare_name_never_chooses_planted_binary_in_current_directory` 不再啟動系統 Notepad（改啟動 `ping.exe -n 30 127.0.0.1`，解析檢查對 `notepad.exe` 與 `ping.exe` 都保留）；live 測試的鎖改用 `notepad_launch_lock()`，鎖被毒化時仍取得 guard。成因：當日 Notepad 啟動即崩潰，以及已開著 Notepad 時 `attach_if_running: false` 必拒，見 ADR-009 | 上游 launch live 測試不依賴系統 Notepad 的存活，且單一測試 panic 不會讓同群測試連帶 `PoisonError` |
| `crates/windows/src/system/launch_path.rs` | launch 與 CLI 檔案參數共用 core 判斷，NT 前綴只接受絕對磁碟路徑；launch 識別字拒絕 UNC、`\\?\UNC\`、`\\.\` 等裝置命名空間與其正斜線寫法（只認 `X:\` 與 `\\?\X:\`），在 `is_file` 之前回 `INVALID_ARGS`；`is_file` 對 `\\server\...` 會發 SMB 而洩漏 NetNTLM，並可執行遠端 exe。獨立覆核發現，見 ADR-011 | 上游 `launch_identifier` 與 `resolve_executable` 同樣在碰檔案系統前拒絕非本機磁碟路徑，且網路路徑測試通過 |
| `crates/windows/src/system/launch_path_network_tests.rs` | 新增網路與裝置路徑拒絕測試：識別字、`resolve_executable`（確認回 `INVALID_ARGS` 而非 `APP_NOT_FOUND`，即未碰檔案系統）、偵測函式與本機 verbatim 路徑仍可用 | 上游以符合 400 行限制的佈局保留相同覆蓋 |
| `crates/windows/src/system/private_file/dacl.rs` | 驗證既有 ACL 時接受 Owner Rights 與 Creator Owner SID；建立時的授權集合不變 | 上游涵蓋這兩種 SID 並仍拒絕其他主體，私有檔案 ACL 測試通過 |
| `crates/windows/src/system/private_file/tests.rs` | private-file scratch 目錄明設 ACL，包含 Owner Rights 與目前使用者 | 上游 fixture 可建立與驗證相同安全邊界的私有目錄 |
| `crates/windows/src/system/runtime/mod.rs` | 新增輸入桌面附掛 helper；OpenInputDesktop 或 SetThreadDesktop 失敗不回報給呼叫端 | 上游提供等效桌面處理並通過隔離桌面測試，再移除本地 helper 及其使用點 |
| `crates/windows/src/system/wait_tests.rs` | 選單逾時測試改為只釘方向性前綴（zero-poll 後綴由 `wait_expiry_detail_tests.rs` 確定性覆蓋）；測試檔行數受 400 行上限約束 | 上游測試在同類 Windows 負載下以原斷言穩定通過，或後綴組成改由真實桌面以外的方式覆蓋 |
| `crates/windows/src/system/window/window_ops_tests.rs` | 持續不一致的重試預算測試 deadline 由 10 s 改為 60 s，不改變重試次數或拒絕碼 | 上游在同類 Windows 負載下以原預算穩定通過 |
| `crates/windows/src/system/signal/wait_event_live_tests.rs` | app-terminated 成功路徑的等待上限由 10 s 改為 60 s（只影響逾時失敗路徑的耗時，健康路徑立即返回） | 上游測試在同類 Windows 負載下以原預算穩定通過 |
| `crates/windows/src/system/capture/capture_envelope_parity_tests.rs` | scaled display 截圖測試的 deadline 由 10 s 改為 60 s（健康路徑不變，僅斷言 scale 與 owner 對應） | 上游測試在同類 Windows 負載下以原預算穩定通過 |
| `crates/windows/src/system/signal/signal_inventory_command_live_tests.rs` | 無轉換事件的等待上限由 1500 改為 3000 ms；基線計數測試改以測試端預先擷取（60 s 預算）的基線經 `with_event_baseline` 種入，不再依賴等待內的即時擷取在 3 s 內完成；可重試失敗證據測試改為以 60 s 預算預先擷取的失敗經 `with_event_baseline` 種入（不再依賴 3 s 內強制輪詢失敗），斷言改為「可重試碼之一」（後續真實輪詢的 TIMEOUT 會覆寫 last_error） | 上游測試在同類 Windows 負載下以原預算穩定通過，或有不靠放寬預算的同步方式 |
| `crates/windows/src/system/window/window_enum.rs` | EnumWindows 前附掛桌面、清除 LastError，回 0 但沒有錯誤時不報失敗；測試改用 `mem::take` | 上游正確區分停止列舉與 Win32 錯誤，隔離桌面列舉及 clippy 通過 |
| `crates/windows/src/tree/fixture/fixture_clipboard.rs` | 延遲剪貼簿 owner 改用既有有界重試，容忍其他程式暫時占用剪貼簿 | 上游處理相同占用競態，延遲剪貼簿 fixture 測試通過 |
| `crates/windows/src/tree/fixture/fixture_menu.rs` | menu 子程序與執行緒附掛桌面並 bootstrap，另輸出子程序診斷行 | 上游 menu fixture 在隔離桌面能正常就緒，移除診斷輸出後仍可定位失敗 |
| `crates/windows/src/tree/fixture/fixture_modal.rs` | modal fixture 執行緒建立視窗前附掛輸入桌面 | 上游 modal fixture 在隔離桌面可建立及觀測視窗 |
| `crates/windows/src/tree/fixture/fixture_pattern_gdi.rs` | pattern 視窗與 BitBlt 前附掛桌面，失敗時帶 Win32 error | 上游 fixture 支援相同桌面環境及失敗診斷，pattern capture 測試通過 |
| `crates/windows/src/tree/fixture/fixture_window.rs` | 一般 Win32 fixture 視窗執行緒先附掛輸入桌面 | 上游 fixture 在隔離桌面可建立並觀測視窗 |
| `crates/windows/src/tree/fixture/mod.rs` | bootstrap、COM apartment 與 stalled fixture 入口附掛輸入桌面 | 上游提供等效 fixture 初始化，正常及 stalled 視窗測試通過 |
| `crates/windows/src/tree/hit_test/envelope_tests.rs` | occluded headed auto-wait 測試預算由 80 改為 1000 ms，仍驗證 ACTION_FAILED 與 not_delivered | 上游在同類 Windows 負載下用原預算穩定驗證相同合約，或改用確定性同步 |
| `crates/windows/src/tree/surface_inventory_tests.rs` | 無視窗程序 fixture 改找任何可讀 generation token 的程序，移除固定系統程序名稱限制 | 上游 fixture 不依賴特定程序名稱且仍驗證空 inventory |
| `crates/windows/src/tree/walk/cache_tests.rs` | 測試接受 Batch 或 Skip，放寬原本只能 Batch 的斷言；此差異降低策略驗證強度 | 上游以可控制的 fixture 明確驗證 cache policy，或能證明兩種結果都符合測試前提 |
| `src/command_policy/mod.rs` | Windows `preflight` 先共用檔案路徑檢查拒絕 UNC 與裝置命名空間，再呼叫 `mcp::restrictions::enforce`：由 MCP 伺服器啟動的子行程在動作前拒絕 force、launch、batch、檔案路徑等超出限制的指令；batch 的每個項目走同一個 `preflight`，所以 batch 無法繞過 | 上游 MCP 伺服器在子行程端強制等效限制，或本 fork 取回上游版本 |
| `src/mcp/call.rs` | 子行程啟動時帶入 `ChildSetup`（全域旗標加上 MCP 限制的環境變數），取代單純的 `globals`（函式參數不得超過 5 個） | 上游 MCP 伺服器把限制傳給子行程，或取回上游版本 |
| `src/mcp/catalog.rs` | 工具清單與 schema 依 `Restrictions` 過濾（隱藏 launch、batch、session gc、`force`、`cwd`、未開啟的檔案與 cdp 參數）；相對檔案路徑改寫到 session 輸出目錄；trace export 預設輸出路徑落在該目錄 | 上游 MCP 目錄自行過濾危險工具與參數，或取回上游版本 |
| `src/mcp/catalog_tests.rs` | 測試輔助 `tools()` 改為全開限制；新增「每個路徑型參數都有檔案限制或不提供」與「預設目錄不含 launch、batch、session gc」測試 | 隨 `src/mcp/catalog.rs` 取回上游而移除 |
| `src/mcp/confinement.rs` | 新檔：MCP 限制的路徑與參數檢查——輸出目錄內判定（輸出目錄不存在或無法解析即拒絕；逐層不跟隨連結地走過輸出目錄之下的每個部件，symlink 或 junction 等 reparse point 即使懸空也拒絕；拒絕保留裝置名與尾端點或空白與串流）、launch 環境變數（inspector 類與 PATH 類）與 debug 端點的子字串掃描，見 ADR-011 的推送前審查修正 | 隨 `src/mcp/restrictions.rs` 取回上游或上游自帶等效檢查而移除 |
| `src/mcp/confinement_tests.rs` | 新檔：confinement 的測試，含環境變數、嵌入式開關、PATH 類變數、close-app force、裝置名與不存在的輸出目錄、懸空 junction 與以 junction 命名的輸出檔 | 隨 `src/mcp/confinement.rs` 移除 |
| `src/mcp/confinement_env_tests.rs` | 新檔：launch 環境變數（程式載入、profiler、Git 設定注入）與 Firefox debugger 參數的測試，先紅後綠，含誤擋邊界 | 隨 `src/mcp/confinement.rs` 移除 |
| `src/mcp/exposure.rs` | 新檔：決定哪些工具與參數在某組限制下公開、哪些參數是檔案路徑、相對路徑如何改寫（`catalog.rs` 已接近 400 行） | 隨 `src/mcp/catalog.rs` 取回上游而移除 |
| `src/mcp/exposure_tests.rs` | 新檔：公開範圍與路徑改寫測試 | 隨 `src/mcp/exposure.rs` 移除 |
| `src/mcp/mod.rs` | `mcp` 子命令加四個 `--allow-*` 旗標；`serve` 解析限制並傳給子行程，session 不存在時拒絕 `--allow-file-write`；INSTRUCTIONS 說明預設關閉與「不是沙盒」；`Server` 改持有 `ChildSetup` | 上游 MCP 伺服器提供等效旗標與說明，或取回上游版本 |
| `src/mcp/mod_tests.rs` | 測試伺服器改用預設（全關）限制與 `ChildSetup` | 隨 `src/mcp/mod.rs` 取回上游而移除 |
| `src/mcp/restrictions.rs` | 新檔：MCP 限制的資料模型、環境變數編解碼，與對 `Commands` 的檢查（force、launch、batch、session gc、檔案路徑、DevTools 與 inspector 參數）；路徑檢查含 `..`、UNC、junction 與 symlink 逃逸 | 上游以型別層限制 MCP 可達的指令並涵蓋相同拒絕，或取回上游版本 |
| `src/mcp/restrictions_tests.rs` | 新檔：每項限制的測試，含 batch 項目、junction 逃逸與環境往返 | 隨 `src/mcp/restrictions.rs` 移除 |
| `src/mcp/startup.rs` | 新檔：`mcp` 的 `--allow-*` 旗標定義與限制解析（`mod.rs` 的行數預算） | 隨 `src/mcp/mod.rs` 取回上游而移除 |
| `src/mcp/startup_tests.rs` | 新檔：旗標與 session 輸出目錄解析測試 | 隨 `src/mcp/startup.rs` 移除 |
| `src/tests/cli_batch_session_end_tests.rs` | 從 `cli_process.rs` 拆出 batch session-end 測試以符合 400 行限制 | 上游以符合 source rules 的佈局保留相同 session-end 覆蓋 |
| `src/tests/cli_process.rs` | Windows CLI scratch 目錄明設私有 ACL；batch session-end 測試拆到相鄰模組並公開 helper | 上游提供可用的 Windows 隔離目錄，並保留 batch 測試覆蓋及 400 行限制 |
| `src/tests/mcp_process.rs` | 新增行程層測試：預設清單不含 launch、batch、session gc；force 與未列出的工具在執行前被拒；子行程在受限環境下回 `POLICY_DENIED`；`--allow-file-write` 無 session 時結束 | 上游行程測試涵蓋同樣的預設關閉行為 |
| `src/tests/screen_sample.rs` | 螢幕像素測試改用固定大小 `as_chunks`，對齊 clippy 要求 | 上游保持相同 BGR 像素比對語意，螢幕取樣測試與 clippy 通過 |
| `crates/core/src/trace/trace_artifacts_dispatch_budget_tests.rs` | `slow_pre_capture_preserves_dispatch_budget` 改用 `VirtualClock`，慢速截圖以虛擬時間推進，並斷言派送時預算不低於 500 ms 減 50 ms；原測試以牆鐘比較 500 ms 預算，預派送階段的真實 I/O 在慢速 runner 上會吃掉預算而誤判逾時 | 上游同名測試改用可控時鐘或等效的確定性斷言，且 Windows core 測試通過 |
| `crates/core/src/ref_action/presentation_shared_cursor_tests.rs` | `a_command_in_another_session_travels_without_waiting` 改為由本執行緒持有另一工作階段游標鎖、不再以 400 ms 牆鐘門檻判定；完成即證明未等待該鎖 | 上游同名測試採用等效的確定性斷言，且 Windows core 測試通過 |
| `src/command_policy/file_path.rs` | Windows CLI 檔案路徑使用 core 共用詞法檢查，拒絕 UNC、NT 遠端前綴與裝置命名空間 | 上游提供相同路徑限制與測試 |
| `src/command_policy/file_path_tests.rs` | 保留本機相對、磁碟與延伸磁碟路徑，並驗證 UNC、裝置路徑與 batch 檔案參數拒絕的測試 | 上游提供相同路徑限制與測試 |
| `src/command_policy/tests.rs` | Windows UNC 路徑在權限檢查與檔案存取前拒絕的回歸測試 | 上游提供相同路徑限制與測試 |
| `src/tests/main_tests.rs` | 全域 trace 與 debug 截圖路徑在命令初始化前拒絕 UNC 的測試 | 上游提供相同路徑限制與測試 |
| `src/main.rs` | Windows 全域 trace、debug 截圖與指令檔案路徑在命令初始化前檢查 | 上游在相同邊界拒絕 UNC 與裝置路徑 |
| `crates/core/src/lib.rs` | 匯出共用的 Windows 網路與裝置路徑判斷 | 上游提供等效共用判斷 |
| `crates/core/src/system/mod.rs` | 註冊不存取檔案系統的 Windows 路徑判斷模組 | 上游提供等效共用判斷 |
| `crates/core/src/system/windows_path.rs` | CLI 檔案與 launch 共用路徑判斷；NT 與延伸前綴只接受絕對磁碟路徑 | 上游提供等效路徑限制 |
| `crates/core/src/system/windows_path_tests.rs` | 拒絕 NT UNC、GLOBALROOT、裝置與相對磁碟路徑，保留本機路徑 | 上游提供等效路徑測試 |
