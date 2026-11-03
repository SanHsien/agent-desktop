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

既有 2026-11-02 快照稱 Windows lib 共 8 筆失敗，但 repo 僅記下上述 1 筆完整名稱；其餘 7 筆的完整名稱與原失敗輸出未收錄。這份診斷不能確認其他失敗的原因，也不能以具名測試通過代表完整 Windows lib 通過。

## 本機門禁範圍

`cargo fmt --all -- --check` 與 Windows host 四套件的 `cargo clippy --all-targets -- -D warnings` 均 exit 0。`cargo test -p agent-desktop-windows --lib system::launch::` 為 exit 0、24 passed／0 failed，耗時 39.50 秒，上限 300 秒。這是 launch 群組，並非原 8 筆失敗清單或完整 Windows lib。

`pwsh -NoProfile -File tools/dev_check.ps1 -Full` 為 exit 0，46 筆衛生測試及 33 檔差異清單核對通過；npm native binary 不存在，腳本略過狀態檢查。額外執行的檔案大小／註解、plan reference 與 stale-ref constructor source rules 通過。`bash scripts/check-architecture.sh` exit 1，錯誤為 `self-test: G11 did not fire for its planted case in crates/core/src/tree/mod.rs`。從上述 SHA 讀取未修改的 `scripts/check_architecture.py`，只執行其隔離 fixture `self_test()` 也得到相同失敗；此重現不依賴工作樹的測試修正。架構檢查器與 core tree 檔案均無修改。
