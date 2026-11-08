# Windows lib 測試的碼頁診斷

## 2026-11-03：lease 子程序環境傾印

程式基準：`846d047f7894de8b575fc9ba65dd1c232b460271`。具名測試為 `system::launch::launch_lease_isolation_tests::a_launched_process_never_inherits_the_lease_handoff_key`。

測試以 `CreateProcessW` 傳入移除 lease handoff key 的環境區塊，再讀取子程序 `cmd /C set` 的輸出。`CREATE_UNICODE_ENVIRONMENT` 指定的是輸入環境區塊編碼，不會讓 cmd 的輸出變成 UTF-8。原先的 `read_to_string` 因此對輸出碼頁有錯誤假設。

```text
Unicode 環境區塊 -> cmd 以輸出碼頁寫檔 -> read_to_string 要求 UTF-8
                                      -> 碼頁 950 的非 ASCII 值解碼失敗
Unicode 環境區塊 -> cmd /U 寫 UTF-16LE -> 嚴格 UTF-16LE 解碼
```

初始殼層的 `chcp` 為 950，`[Console]::OutputEncoding.WebName` 為 `utf-8`。一般工具殼層中的原測試通過；另開隱藏、獨立的 Windows console，先用 System32 的 `chcp.com 950` 設定碼頁再啟動 cargo，加入合成非 ASCII 環境值後，原測試 exit 101，錯誤為 `stream did not contain valid UTF-8`。不能用 PowerShell 的輸出編碼推定另一個 cmd 子程序的檔案編碼。

控制實驗只傳入 `SystemRoot` 與合成變數 `PROBE=測試`，不傾印真實環境。另開 cmd 在設定碼頁後輸出該變數，觀察值如下：

| cmd 輸出模式 | 碼頁 950 | 碼頁 65001 |
| --- | --- | --- |
| 預設 | `PROBE=` 後為 `b4 fa b8 d5`，非有效 UTF-8 | `PROBE=` 後為 `e6 b8 ac e8 a9 a6`，有效 UTF-8 |
| `/U` | UTF-16LE，可還原原值 | 相同 UTF-16LE bytes，可還原原值 |

修正位於 `crates/windows/src/system/launch_lease_isolation_tests.rs`，只改測試：cmd 使用 `/D /U`，停用 AutoRun 並明確指定 Unicode 輸出；讀檔後先驗證完整雙位元組，再嚴格解碼 UTF-16LE。合成值 `測試🚀` 經過繼承環境區塊與真實子程序後必須完整保留。空 overrides、繼承 `SystemRoot` 與 lease handoff key 不得存在的檢查均保留，產品 launch 邏輯不變。

回歸指令為 `cargo test -p agent-desktop-windows --lib a_launched_process_never_inherits_the_lease_handoff_key`。加入合成值但尚使用原解碼方式時，碼頁 950 為 exit 101、0 passed／1 failed；修正後碼頁 950 與 65001 各為 exit 0、1 passed／0 failed。

既有 2026-11-02 快照稱 Windows lib 共 8 筆失敗，但當時 repo 僅記下上述 1 筆完整名稱；其餘 7 筆的完整名稱與原失敗輸出未收錄於 repo。

## 2026-11-07：本機 Windows lib 測試失敗重現觀察與分類

經於 2026-11-07 在 Windows 11 本機以整批並行與單筆對照實測，觀察到以下失敗項目與 2026-11-07 本機重現輸出。逐筆分類如下：

### 1. 測試缺陷（已修，既有 commit `85a1e5a2`）：1 筆
- `system::launch::launch_lease_isolation_tests::a_launched_process_never_inherits_the_lease_handoff_key`
  - `85a1e5a2` 記錄的先紅輸出：`stream did not contain valid UTF-8`
  - 成因：`cmd.exe /C set` 在繁體中文 Windows 預設使用碼頁 950 寫檔，原測試以 `read_to_string` 要求 UTF-8 造成非 ASCII 變數解碼失敗。
  - 處置：既有 commit `85a1e5a2`（2026-11-03）已修正，改傳 `/D /U` 並嚴格以 UTF-16LE 解碼，同時注入非 ASCII 合成 probe `\u{6e2c}\u{8a66}\u{1f680}` 保證跨語系正確性。先紅後綠證據見 `85a1e5a2`。

### 2. 測試環境隔離缺陷（隔離漏洞已修；11-02 成因仍未證實；taskbar 未實測）：2 筆
- `system::capture::capture_envelope_parity::routing::clipboard_image_default_path_travels_private_seam_reparse_and_owner`
  - 2026-11-07 本機重現輸出：`reparse tmp must refuse private write: Object {"height": Number(2), "path": "...", ...}`
- `system::shell::shell_surface_open::command_live_tests::opening_the_taskbar_round_trips_without_raising_anything`
  - 2026-11-07 本機重現輸出：`the snapshot consumes the open surface's identity: Adapter(AdapterError { code: Internal, message: "private file DACL grants an untrusted principal", ...})`
  - 觀察與處置：隔離漏洞已修；11-02 成因仍未證實；taskbar 未實測。
    - 成因分析：`crates/windows/src/system/capture/capture_test_support.rs` 的 `HomeIsolation::enter` 原僅重設 `HOME` 與 `USERPROFILE`，未清除 `AGENT_DESKTOP_HOME`。依 `crates/core/src/storage/state_root.rs:33`，若執行環境設定了 `AGENT_DESKTOP_HOME`，將優先使用該外部目錄，導致測試脫離隔離 scratch 目錄，使 `clipboard_image_default_path_travels_private_seam_reparse_and_owner` 寫入外部目錄而繞過隔離目錄中的 `tmp` junction。2026-11-02 維護者本機是否設有該變數未曾記錄，故其是否為 11-02 失敗原因仍未證實；taskbar 該筆亦未在外部設定該變數下單獨重測。
    - 處置：在 `HomeIsolation::enter` 同步清除 `AGENT_DESKTOP_HOME` 並於 `Drop` 還原，並於 `capture_routing_parity_tests.rs` 新增單元測試 `home_isolation_isolates_state_root_when_external_agent_desktop_home_is_set` 完成先紅後綠實證：
      - 先紅輸出：`thread 'system::capture::capture_envelope_parity::routing::home_isolation_isolates_state_root_when_external_agent_desktop_home_is_set' panicked at crates\windows\src\system\capture\capture_routing_parity_tests.rs:150:9: state root must land inside HomeIsolation root, got: %TEMP%\external-home-44344`
      - 後綠輸出：`test system::capture::capture_envelope_parity::routing::home_isolation_isolates_state_root_when_external_agent_desktop_home_is_set ... ok`
      - 修復後即使外部環境設定 `AGENT_DESKTOP_HOME`，`clipboard_image_default_path_travels_private_seam_reparse_and_owner` 亦穩定通過（exit 0）。

### 3. 測試環境假設（並行負載逾時 / 桌面動畫逾時）：4 筆
- `system::capture::capture_envelope_parity::tests::screenshot_path_and_inline_data_shapes_match_core_serialization`
  - 2026-11-07 本機重現輸出：`screenshot inline: Adapter(AdapterError { code: Timeout, message: "Operation exceeded its deadline", details: Some(Object {"elapsed_ms": Number(5255), "timeout_ms": Number(5000)}) })`
  - 觀察：1548 筆並行測試時，螢幕截圖與 D3D 資源爭用導致逾時（5255ms > 5000ms）。單獨重跑回報耗時 2.21s 通過。
- `system::capture::capture_modern::tests::frame_wait_deadline_expiry_falls_back_to_legacy`
  - 2026-11-07 本機重現輸出：`precedence must still succeed via Legacy after a modern timeout: AdapterError { code: Timeout, ... }`
  - 觀察：全套並行測試時圖形擷取幀等待逾時。單獨重跑回報耗時 0.97s 通過。
- `system::signal::signal_surfaces::tests::an_open_menu_produces_a_menu_surface_signal_and_no_event_while_it_stays_open`
  - 2026-11-07 本機重現輸出：`assertion left == right failed: exactly one Menu surface must appear: [] (left: 0, right: 1)`
  - 觀察：子程序 context menu 彈出時受並行進程視窗競爭影響未能及時捕獲 Menu 表面。單獨重跑回報耗時 0.92s 通過。
- `system::shell::shell_surface_open::command_live_tests::opening_the_action_center_round_trips_into_the_snapshot`
  - 2026-11-07 本機重現輸出：`the command opens the action-center: AdapterError { code: Timeout, message: "Operation exceeded its deadline", details: Some(Object {"elapsed_ms": Number(5196), "timeout_ms": Number(5000)}) })`
  - 觀察：Action Center 開啟動畫在並行負載下耗時略逾 5s。單獨重跑回報耗時 9.62s 通過。

### 4. 測試環境假設（真實桌面 Start Menu 動畫與焦點搶佔競爭）：1 筆
- `system::shell::shell_surface_open::command_live_tests::opening_the_start_menu_round_trips_into_the_snapshot`
  - 2026-11-07 本機重現輸出：`the snapshot consumes the open surface's identity: Adapter(AdapterError { code: WindowNotFound, message: "The 'start-menu' shell surface is not open on this desktop", ...})`
  - 觀察：透過 Win 鍵開啟 Windows 11 Start Menu（`StartMenuExperienceHost.exe`）後，若桌面焦點因背景視窗活動移動或 DWM 視窗被 cloaked，在快照讀取時表面已自動收合。在隔離環境下連續 5 次執行有 4 次通過（回報耗時 21s~40s，另有 1 次執行在外層指令 90 秒逾時中止，非測試自身時限，該次未產生上述 WindowNotFound 輸出）；碼頁 950 全套測試中順利通過。

## 雙碼頁驗證結果

依驗收條件，在碼頁 950 與 65001 環境下分別執行 `scripts/cargo-test-isolated-home.sh test -p agent-desktop-windows --lib`：
- **碼頁 950**：全套 1548 筆執行 exit 0。
  回報：exit 0，1542 passed／0 failed，6 ignored，約 176 秒（未保存原始日誌）。
- **碼頁 65001**：全套 1548 筆執行 exit 101。
  回報：exit 101，1541 passed／1 failed（Start Menu），6 ignored（未保存原始日誌）。
  單獨重跑該 Start Menu 測試 exit 0，回報：1 passed／0 failed，約 27.77 秒（未保存原始日誌）。
- 本次未觀察到語系相關產品缺陷。
- 衛生門禁：`pwsh -NoProfile -File tools/dev_check.ps1 -Full` exit 0 通過。
- 格式與代碼風格：`cargo fmt --all -- --check` 與 `cargo clippy -p agent-desktop-core -p agent-desktop-windows -p agent-desktop -p agent-desktop-ffi --all-targets -- -D warnings` 皆 exit 0 通過。
- 二進位與 FFI 門禁：`cargo test -p agent-desktop -p agent-desktop-ffi --tests` exit 0 通過。

## 本機門禁範圍

`cargo fmt --all -- --check` 與 Windows host 四套件的 `cargo clippy --all-targets -- -D warnings` 均 exit 0。`cargo test -p agent-desktop-windows --lib system::launch::` 為 exit 0、24 passed／0 failed，耗時 39.50 秒，上限 300 秒。這是 launch 群組，並非原 8 筆失敗清單或完整 Windows lib。

`pwsh -NoProfile -File tools/dev_check.ps1 -Full` 為 exit 0，46 筆衛生測試及 33 檔差異清單核對通過；npm native binary 不存在，腳本略過狀態檢查。額外執行的檔案大小／註解、plan reference 與 stale-ref constructor source rules 通過。`bash scripts/check-architecture.sh` exit 1，錯誤為 `self-test: G11 did not fire for its planted case in crates/core/src/tree/mod.rs`。從上述 SHA 讀取未修改的 `scripts/check_architecture.py`，只執行其隔離 fixture `self_test()` 也得到相同失敗；此重現不依賴工作樹的測試修正。架構檢查器與 core tree 檔案均無修改。
