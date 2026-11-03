# 產品檔案與上游的差異

Base: `9eb9b296192fcb70bda25e918cd6dd66abfa3d0d`

基準為上游 `main` 的 `Video remove (#421)`。本表只列 `crates/`、`src/`、`apps/`、`npm/` 的差異；fork 維護文件、工具與 workflow 見 [FORK.md](../FORK.md)。來源為 `8d372752` 的 Windows 修正、`0f7e8e65` 的格式整理及 `a67bf683` 的測試拆檔。

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
| `crates/core/src/session/tree.rs` | 非 Unix session 根目錄也經 `ensure_private`，避免只保護 trace 子目錄 | 上游在建立 session 時同樣保護根目錄，Windows session 測試通過 |
| `crates/ffi/src/actions/result.rs` | 釋放 action result 時用 `as_mut` 檢查空指標；單元測試移到相鄰檔案以符合 400 行限制 | 上游涵蓋空 result 與 post-state 釋放，且檔案大小及 FFI 測試通過 |
| `crates/ffi/src/actions/result_tests.rs` | 從 `result.rs` 拆出的 action result 轉換及釋放測試，包含空 result 測試 | 上游以符合 400 行限制的佈局保留相同測試覆蓋 |
| `crates/ffi/tests/common/mod.rs` | Windows FFI 隔離目錄移除繼承 ACL，授權 SYSTEM、Administrators 與 Owner Rights | 上游 fixture 能在 Windows 原生環境建立被私有檔案檢查接受的隔離目錄 |
| `crates/ffi/tests/common/win32_fixture.rs` | FFI Win32 fixture 執行緒先附掛輸入桌面 | 上游 fixture 在隔離桌面環境可建立及觀測視窗，FFI 實機測試通過 |
| `crates/windows/Cargo.toml` | 啟用 `Win32_System_StationsAndDesktops` 供輸入桌面附掛使用 | 上游提供等效桌面處理，或附掛呼叫全部移除後 Windows 建置通過 |
| `crates/windows/src/input/clipboard/clipboard_bytes.rs` | UTF-16 解碼改用固定大小 `as_chunks`，對齊 clippy 要求 | 上游解碼保留尾端不完整位元組的行為，且 clippy 與剪貼簿測試通過 |
| `crates/windows/src/input/clipboard/clipboard_files_tests.rs` | HDROP 測試的 UTF-16 解碼同步改用 `as_chunks` | 上游測試具有相同對齊與 round-trip 覆蓋，clippy 通過 |
| `crates/windows/src/lib.rs` | 匯出 `attach_input_desktop` 給跨 crate 的 fixture 使用 | 上游有等效公開入口，或跨 crate 的附掛呼叫不再需要 |
| `crates/windows/src/system/capture/capture_test_support.rs` | capture 測試的隔離目錄明設私有 ACL，避免環境繼承權限影響測試 | 上游隔離目錄能通過 Windows 私有目錄驗證與 capture 測試 |
| `crates/windows/src/system/capture/capture_window.rs` | 空影格檢查改用固定大小像素區塊，對齊 clippy 要求 | 上游保留空影格判定語意，capture 測試與 clippy 通過 |
| `crates/windows/src/system/capture/gdi_surface.rs` | GDI 建立螢幕 DC 前先附掛輸入桌面 | 上游有等效桌面處理，隔離環境 GDI capture 測試通過 |
| `crates/windows/src/system/cursor_overlay/text_tests.rs` | 用 `fill` 填入卡片像素，取代逐像素賦值以符合 clippy | 上游測試保持相同像素輸入且 clippy 通過 |
| `crates/windows/src/system/interaction_lease/directory_tests.rs` | 缺少 SeRestorePrivilege 時跳過異主目錄測試，並忽略部分清理錯誤；這會減少該環境的測試覆蓋 | 上游 fixture 可在無特權環境驗證拒絕行為，或保留可稽核的跳過理由及清理方式 |
| `crates/windows/src/system/interaction_lease/tests.rs` | lease scratch 目錄明設私有 ACL，避免繼承權限干擾 | 上游 scratch 目錄在 Windows 原生環境通過 lease 私有目錄測試 |
| `crates/windows/src/system/private_file/dacl.rs` | 驗證既有 ACL 時接受 Owner Rights 與 Creator Owner SID；建立時的授權集合不變 | 上游涵蓋這兩種 SID 並仍拒絕其他主體，私有檔案 ACL 測試通過 |
| `crates/windows/src/system/private_file/tests.rs` | private-file scratch 目錄明設 ACL，包含 Owner Rights 與目前使用者 | 上游 fixture 可建立與驗證相同安全邊界的私有目錄 |
| `crates/windows/src/system/runtime/mod.rs` | 新增輸入桌面附掛 helper；OpenInputDesktop 或 SetThreadDesktop 失敗不回報給呼叫端 | 上游提供等效桌面處理並通過隔離桌面測試，再移除本地 helper 及其使用點 |
| `crates/windows/src/system/signal/signal_inventory_command_live_tests.rs` | 無轉換事件的等待上限由 1500 改為 3000 ms，降低本機負載造成的逾時 | 上游測試在同類 Windows 負載下以原預算穩定通過，或有不靠放寬預算的同步方式 |
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
| `src/tests/cli_batch_session_end_tests.rs` | 從 `cli_process.rs` 拆出 batch session-end 測試以符合 400 行限制 | 上游以符合 source rules 的佈局保留相同 session-end 覆蓋 |
| `src/tests/cli_process.rs` | Windows CLI scratch 目錄明設私有 ACL；batch session-end 測試拆到相鄰模組並公開 helper | 上游提供可用的 Windows 隔離目錄，並保留 batch 測試覆蓋及 400 行限制 |
| `src/tests/screen_sample.rs` | 螢幕像素測試改用固定大小 `as_chunks`，對齊 clippy 要求 | 上游保持相同 BGR 像素比對語意，螢幕取樣測試與 clippy 通過 |
