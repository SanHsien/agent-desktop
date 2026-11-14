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

## 2026-11-15：時限敏感測試剩餘項的量測與處置

程式基準：`87191437`。機器：Windows 11 Pro 10.0.26300，24 邏輯處理器，碼頁 950，環境已設 `AGENT_DESKTOP_HOME`。量測期間桌面無人操作。原始日誌未收進 repo；下列數字都是當次實際輸出。

### 安靜桌面的基線（未改動）

指令：`"C:\Program Files\Git\bin\bash.exe" scripts/cargo-test-isolated-home.sh test -p agent-desktop-windows --lib`，連跑 3 次。

| 次 | exit | passed | failed | ignored | 測試耗時 |
| --- | --- | --- | --- | --- | --- |
| 1 | 0 | 1490 | 0 | 6 | 142.39 s |
| 2 | 0 | 1490 | 0 | 6 | 126.46 s |
| 3 | 0 | 1490 | 0 | 6 | 136.37 s |

ROADMAP 該列點名的每一筆（剪貼簿兩筆、截圖四筆、殼層 UIA 六筆、`window_opened_fires_for_a_modal…`、`two_simultaneously_open_sheets…`、`contending_holder…`）三次皆 `ok`：安靜桌面的 3 連綠在改動前就出現過一次。但它不穩定，見文末「修正後安靜桌面全量」第 3 次。

### 壓力下重現（24 個 Python 忙迴圈程序，未改動的測試加暫時診斷輸出）

全量 2 次：

| 次 | 失敗數 | 失敗項目與訊息 |
| --- | --- | --- |
| 1 | 1 | `shell_surface::tests::start_menu_opens_and_identity_roots_a_tree`：`the start accelerator raises its surface: Timeout`，`elapsed_ms 24499`／`timeout_ms 10000` |
| 2 | 6 | `shell_surface::tests::action_center_opens_roots_and_closes_cloaked`（`elapsed_ms 21127`／`10000`）；`shell_surface::tests::immersive_surface_absent_from_enumeration_but_yielded_by_uia_root`（`21991`／`10000`）；`screenshot::tests::adapter_screenshot_delegates_to_orchestration`（`10173`／`10000`）；`screenshot::tests::exact_window_without_process_instance_is_invalid_args_not_delivered`（`12325`／`10000`）；`live_surface_tests::surface_dismissed_fires_when_the_already_open_modal_closes_mid_wait`（`UI Automation could not resolve a window root`，`COM HRESULT 0x80131505 (UIA_E_TIMEOUT)`）；`system::wait::tests::already_in_the_requested_state_returns_immediately_reading_the_predicate_once`（predicate 被讀 2 次，預期 1 次） |

只跑名稱含 `clipboard` 的 53 筆（含 1 筆 ignored）、同樣壓力下共 345 輪，5 輪失敗：

| 測試 | 次數 | 訊息 |
| --- | --- | --- |
| `fixture_clipboard::tests::contending_holder_is_a_second_process_that_blocks_open` | 1 | `holder releases on request: "clipboard holder did not exit after release"` |
| 同上 | 1 | `OpenClipboard must succeed after the holder releases` |
| `input::clipboard::tests::empty_clipboard_returns_ok_none_distinct_from_transport_error` | 2 | `release: "clipboard holder did not exit after release"` |
| `clipboard_session::tests::write_open_uses_hidden_owner_while_null_owner_leaves_ownership_unset` | 1 | `OpenClipboard(NULL) must succeed for the negative control` |

### (d) `contending_holder…`：兩個成因，皆已修

```text
測試要的事實：holder 放開剪貼簿
原本等的事：  holder 子程序在 5 秒內結束        <- 兩者在 CPU 壓力下會分開
```

1. **釋放被綁在子程序結束時間上。** `ContendingClipboardHolder::release` 原本寫入 `release` 後輪詢 `try_wait` 5 秒，逾時就 kill 並回 `clipboard holder did not exit after release`。暫時加在 fixture 兩端的時間戳顯示：子程序收到要求後通常在數毫秒內就 `CloseClipboard`、`DestroyWindow` 並呼叫 `std::process::exit(0)`，但作業系統把那個程序收掉的時間不受它控制。有時間戳的 960 次釋放中，等待子程序結束的時間有 2 次落在 4.35 s 與 4.56 s（差一點逾時），2 次超過 5 秒而失敗：15.39 s 與 85.66 s，這兩次子程序都在要求後 10 ms 內記下「即將 exit」。卡住時取樣到該程序只剩 1 條執行緒（其中一次讀到 `WaitReason=Executive`），handle 數停在 111 與 114（正常執行時 193～197），連續取樣 1.4～1.7 秒不變。少掉的 83～86 個與參考傾印中無法複製（`DuplicateHandle` 錯誤 50）的 type index 56 handle 數（84 個）相當；推測是結束流程中逐一登出的 ETW 註冊，**這一點是推論，未證實**，核心端為什麼會停這麼久也未查明。
   修正：`release` 改為等到 holder 的視窗不再是 `GetOpenClipboardWindow()` 的回傳值（上限 30 秒），之後對子程序送 kill 而不等它結束。
2. **單次 `OpenClipboard`。** 剪貼簿歷史服務在每次寫入後會開啟剪貼簿讀內容。實測讓另一個程序以延遲轉譯持有剪貼簿時，佔住剪貼簿的是 `svchost.exe -k ClipboardSvcGroup -s cbdhsvc` 的 `CLIPBRDWNDCLASS` 視窗（本機 `HKCU\Software\Microsoft\Clipboard\EnableClipboardHistory = 1`）。釋放後與對照組的 `OpenClipboard` 只試一次，壓力下會輸掉這個競態。
   修正：新增 `open_clipboard_once_free`（沿用 holder 自己開啟時的有界等待），用在釋放後的開啟與 `write_open_uses_hidden_owner…` 的對照組；`!try_open_clipboard(None)`（必須失敗的那一次）維持單次。

回歸測試（`crates/windows/src/tree/fixture/fixture_clipboard_tests.rs`）：

- `a_holder_whose_process_outlives_its_release_still_reports_the_release`：holder 放開後刻意多活 20 秒。
- `an_open_that_needs_the_clipboard_outwaits_a_transient_opener`：另一條執行緒佔住剪貼簿，等主執行緒回報「單次嘗試已做完」後再多佔 200 ms 才放開；所有開啟都先關閉、執行緒先 join，最後才斷言。

先紅後綠採反向驗證：把 `release` 還原成等子程序結束 5 秒、把 `open_clipboard_once_free` 還原成單次嘗試，兩筆皆 FAILED（前者訊息正是原始失敗的 `clipboard holder did not exit after release`，後者為 `the open must succeed once the transient opener lets go`）；還原修正後兩筆皆 ok。

修正後同樣壓力下只跑 `clipboard` 子集：第一輪 150 輪中 1 輪失敗，失敗的是新測試自己的佔用執行緒用了單次開啟（訊息 `the transient opener takes the clipboard`，同一個競態），改用 `open_clipboard_once_free` 後再跑 60 輪，0 輪失敗。原本的三種失敗訊息在修正後的 210 輪中沒有再出現；修正前的發生率約 1.4%（345 輪 5 次），所以這個輪數只能說「沒有再看到」，確定性的依據是上面兩筆回歸測試。

### (a) 剪貼簿「未結束讀取 worker」守門：未重現，成因仍未確認，未改

- 暫時在 `ensure_no_outstanding_worker` 的拒絕路徑與 `read_format_bytes_on_worker` 的逾時路徑寫診斷檔。345 輪子集加 2 次全量壓力測試中，守門拒絕 0 次。記到 1 次呼叫端放棄 worker，查證是診斷本身造成的（worker 已結束，卡在寫診斷檔才送出結果），不算。
- 會讓守門計數加一的只有 `crates/windows/src/input/clipboard/mod.rs:224`（`WorkerTicket::arm`）。呼叫端只有逾時（`mod.rs:239`）才會在 worker 還活著時返回。測試裡會走到這裡的讀取全部在 `clipboard_test_lock` 之下。
- 測試內唯一預算短到會逾時的是 `clipboard_tests.rs:170` 的 200 ms 讀取。壓力下量到一般讀取的 worker 存活最長 926 ms，所以它確實可能放棄 worker；但那個 worker 只是在重試 `OpenClipboard`，期限一到就結束，而且同一筆測試後面還有受守門保護的讀取，會先讓它自己失敗，不會留到別的測試。
- `capture_test_support.rs:163` 的開場讀取會吞掉自己的錯誤（`_ => None`）。若那次讀取逾時，測試主體第一個剪貼簿操作就會被守門拒絕，而且只會發生在 ROADMAP 點名的那兩筆測試。為了驗證，讓外部程序以延遲轉譯（15 秒才回應 `WM_RENDERFORMAT`）持有剪貼簿再跑這兩筆：兩筆都失敗，但訊息是 `OpenClipboard` 競爭逾時（佔用者是上述 `cbdhsvc` 視窗），不是守門。所以這條路徑在本機沒有重現出 11-12 的症狀。
- 結論：找不到會把 worker 留給別的測試的來源。11-12 的原始失敗輸出沒有保存，無從比對。

### (c) 殼層 UIA live 測試：成因指向產品端呼叫本身，未改

失敗訊息的 `elapsed_ms`（21～24 秒）遠大於測試給的 10 秒：期限檢查是在某一個呼叫返回之後才發生，表示單一呼叫就佔了 20 秒以上。`resolve_immersive`（`crates/windows/src/system/shell/shell_surface_immersive.rs:32`）走訪 UIA 根節點的子元素時不接受期限；另一筆失敗直接帶 `UIA_E_TIMEOUT`。這是 UIA 跨程序呼叫在 CPU 飽和時的行為，期限無法中斷進行中的呼叫。把測試傳入的 10 秒放寬是否足以讓它們通過，這次沒有做對照實驗，所以沒有改。

同一類失敗在安靜桌面也出現了一次（下表第 3 次）：`opening_the_action_center_round_trips_into_the_snapshot` 回 `UI Automation could not read the UIA desktop root's children`、`UIA_E_TIMEOUT`，來源是 `shell_surface_immersive.rs:75`。這筆測試走的是指令自己的預設期限（`DEFAULT_OPERATION_TIMEOUT_MS`，5 秒），不是測試傳入的預算。

### 其他

- `window_opened_fires_for_a_modal…`、`two_simultaneously_open_sheets…`：2 次全量壓力測試都通過，沒有取得失敗訊息，未分析。`wait_event_live_tests.rs` 的 `RECV_TIMEOUT`（20 秒）小於同檔 `WAIT_TIMEOUT_MS`（60 秒），等待若落在兩者之間，測試會先放棄；這是讀程式看到的，未觀察到它造成失敗。
- 壓力下新看到、ROADMAP 沒列的兩筆：`surface_dismissed_fires_when_the_already_open_modal_closes_mid_wait` 與 `already_in_the_requested_state_returns_immediately_reading_the_predicate_once`，各 1 次，未分析。
- 截圖項目（含這次看到的兩筆 10 秒預算）依維護者決定，未動。

### 修正後安靜桌面全量

指令同基線。改動只在剪貼簿 fixture 與其測試；要求的 3 次是第 1～3 次，第 3 次失敗後為了看發生率又多跑 3 次。

| 次 | exit | passed | failed | ignored | 測試耗時 |
| --- | --- | --- | --- | --- | --- |
| 1 | 0 | 1492 | 0 | 6 | 148.46 s |
| 2 | 0 | 1492 | 0 | 6 | 151.58 s |
| 3 | 101 | 1491 | 1 | 6 | 146.03 s |
| 4 | 0 | 1492 | 0 | 6 | 152.49 s |
| 5 | 0 | 1492 | 0 | 6 | 186.49 s |
| 6 | 0 | 1492 | 0 | 6 | 176.45 s |

第 3 次失敗的是 `shell_surface_open::command_live_tests::opening_the_action_center_round_trips_into_the_snapshot`（上面 (c) 的那一筆）。它與這次的改動沒有共用程式碼，但這次沒有另外證明兩者無關。連同基線，安靜桌面 9 次全量中 8 次全綠；「連跑 3 次皆綠」出現過兩段（基線 1～3、修正後 4～6），不是每一段都成立。
