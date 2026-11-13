<p align="center">
  <b>繁體中文</b> · <a href="README.en.md">English</a>
</p>

> **本 repo 是 [SanHsien/agent-desktop](https://github.com/SanHsien/agent-desktop)：全系統 Agent 桌面操控與可靠性加固的維護型 fork，源自 [vercel-labs/agent-desktop](https://github.com/vercel-labs/agent-desktop)（Apache-2.0 授權，保留原作者與上游貢獻者著作權）。** 產品行為與核心無障礙適配以最近一次同步之上游版本為準；本 Fork 專注於 Windows 11 原生深度強化、全系統 Agent 跨平台調用（Antigravity、Claude Code、Codex、Hermes）、PowerShell 逸出防雷、STALE_REF 重試安全合約以及 SSOT 整合。維護規則與同步方法見 [FORK.md](FORK.md) 與 [NOTICE.md](NOTICE.md)。

# agent-desktop

<p align="center"><b>🤖 作業系統原生無障礙樹（Accessibility Tree）桌面操控工具</b><br>
揚棄視覺截圖與座標猜測，賦予 AI Agent 精準、低延遲、省 Token 的 OS 級操作能力</p>

<p align="center">
  <a href="https://github.com/SanHsien/agent-desktop/releases/latest"><img src="https://img.shields.io/github/v/release/SanHsien/agent-desktop?label=release&logo=github" alt="最新 Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="Apache-2.0 License"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-0078D4?logo=windows&logoColor=white" alt="跨平台支援">
  <a href="https://www.npmjs.com/package/agent-desktop"><img src="https://img.shields.io/npm/v/agent-desktop.svg?style=flat&label=npm" alt="npm 版本"></a>
</p>

---

## 💡 為什麼選擇無障礙樹（Accessibility Tree）？

現行主流多模態 Agent 採用「截圖（Screenshot）+ 像素座標定位」的 Computer Use 方案，存在高 Token 消耗、解析度敏感、渲染延遲與座標漂移等痛點。`agent-desktop` 直接查詢作業系統底層無障礙 API（Windows UI Automation、macOS AX、Linux AT-SPI2），建立語意化 UI 節點樹與專屬參照識別碼（Ref ID）：

| 評估維度 | 傳統視覺截圖方案（Vision Computer Use） | `agent-desktop`（原生無障礙樹） |
|---|---|---|
| **Token 消耗** | 每次操作需 1,000 ~ 2,500+ Tokens（High-res 圖片） | 每次快照僅 80 ~ 300 Tokens（純文字 JSON 結構） |
| **執行延遲** | 截圖編碼 + 多模態視覺推理需 2 ~ 6 秒 | 本地原生 OS API 呼叫，**10 ~ 50 毫秒**即時返回 |
| **定位精確度** | 易受縮放（DPI）、主題、解析度漂移影響，常點歪 | 綁定作業系統底層控制代碼（Hwnd / Node），精確度 100% |
| **可交互元素識別** | 依賴視覺反思猜測，不可見元素無法得知 | 原生提供節點角色（Role）、名稱（Name）與可用操作（Actions） |
| **狀態回饋與安全** | 點擊後需再次全螢幕截圖對比像素變化 | 具備確定性狀態碼與 `STALE_REF` 狀態機，安全攔截失效重試 |

```
┌─────────────────────────────────────────────────────────────┐
│                       AI Agent 決策迴路                      │
│                                                             │
│   1. 觀察 (Observe)       2. 操作 (Act)      3. 驗證 (Verify) │
│  ┌──────────────────┐    ┌──────────────┐   ┌─────────────┐ │
│  │ snapshot --app X │───▶│ click '@ref' │──▶│ snapshot -i │ │
│  └──────────────────┘    └──────────────┘   └─────────────┘ │
└───────────────┬───────────────────▲─────────────────────────┘
                │                   │
                ▼                   │
┌─────────────────────────────────────────────────────────────┐
│                      agent-desktop 核心                      │
│                                                             │
│  • JSON Envelope 封裝    • Ref 生命週期管理 (Namespace:Element)│
│  • 密集介面骨架化折疊    • STALE_REF 與 disposition.retry 安全  │
└───────────────┬───────────────────▲─────────────────────────┘
                │                   │
                ▼                   │
┌─────────────────────────────────────────────────────────────┐
│                 OS 原生無障礙層 (Native APIs)                │
│                                                             │
│   [Windows] UI Automation (UIA) / Win32                     │
│   [macOS]   ApplicationServices / AXUIElement               │
│   [Linux]   AT-SPI2 / D-Bus / X11 / Wayland                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 🚀 快速上手

### 1. 安裝方式

#### 方式 A：透過 npm 安裝（推薦）
```bash
npm install -g agent-desktop
agent-desktop version
```
支援 Node.js、Bun、pnpm 或 Yarn。安裝包內建各作業系統原生 binary 與 helper。

#### 方式 B：下載 GitHub Release 原生執行檔（免 Node.js 環境）
至 [SanHsien/agent-desktop Releases](https://github.com/SanHsien/agent-desktop/releases) 下載最新 Windows x64 二進制壓縮包（如 `agent-desktop-v1.0.2-windows-x64.zip`），解壓縮後加入系統 `PATH` 即可。

#### 方式 C：從 Rust 源碼本地編譯
```powershell
git clone https://github.com/SanHsien/agent-desktop.git C:\GitHub\agent-desktop
cd C:\GitHub\agent-desktop
cargo build --release -p agent-desktop
```
二進制檔案生成於 `target/release/agent-desktop.exe`。

---

### 2. 基本迴路：Observe ➔ Act ➔ Observe

列出執行中應用程式、快照 UI 結構、點擊 Ref、再次快照驗證：

```powershell
# 1. 取得應用程式列表
agent-desktop list-apps

# 2. 獲取特定應用程式的可操作節點快照（含 Ref ID）
agent-desktop snapshot --app "記事本" -i

# 3. 針對 Ref 執行點擊（注意：PowerShell 請強制以單引號包裹 Ref！）
agent-desktop click '@s8f3k2p9:e2'

# 4. 輸入文字
agent-desktop type '@s8f3k2p9:e5' "Hello from AI Agent"

# 5. 再次快照確認狀態
agent-desktop snapshot --app "記事本" -i
```

每次呼叫輸出標準 JSON Envelope：
```json
{
  "version": "2.6",
  "ok": true,
  "command": "snapshot",
  "data": {
    "app": "Notepad",
    "window": { "id": "w-10284", "title": "未命名 - 記事本" },
    "snapshot_id": "s8f3k2p9",
    "ref_count": 12,
    "complete": true,
    "tree": {
      "role": "window",
      "name": "未命名 - 記事本",
      "children": [
        {
          "role": "edit",
          "name": "文字編輯器",
          "ref_id": "@s8f3k2p9:e2",
          "available_actions": ["focus", "set-value", "type"]
        }
      ]
    }
  }
}
```

---

## ⚡ Windows 11 與 PowerShell 防雷指南

> [!IMPORTANT]
> **PowerShell Splatting 防雷**：在 PowerShell 中，`@` 字元是陣列/雜湊表展開（Splatting）保留字。若未加引號直接傳入 `@s8f3k2p9:e2`，PowerShell 會將其視為變數，引發語法錯誤。
> 
> ✅ **正確寫法**：一律以**單引號**包裹 Ref！
> ```powershell
> agent-desktop click '@s8f3k2p9:e2'
> agent-desktop type '@s8f3k2p9:e5' 'Hello'
> ```
> ❌ **錯誤寫法**：
> ```powershell
> agent-desktop click @s8f3k2p9:e2    # 拋出語法解析錯誤！
> ```

### 介面過大時的快照優化
當面對肥大視窗（如瀏覽器、複雜 IDE）時，完整快照可能返回數千節點：
- **骨架模式（`--skeleton`）**：折疊無文字且無 Ref 的中間容器，大幅縮減 80% 結構冗餘。
- **區域下鑽（`--root <ref>`）**：只觀測特定面板或表單容器內的子節點。
- **僅可互動（`-i` / `--interactive`）**：過濾唯讀文字與展示用節點，只保留按鈕、輸入框、選單等可操作元件。

---

## 📋 支援命令索引

`agent-desktop` 提供 60 個命令名稱與 56 個即時作業命令（It ships 60 command names, 56 operational commands. The other four (`key-down`, `key-up`, `mouse-down`, `mouse-up`) refuse to run until a daemon can release a held key or button）：

| 分類群組 | 包含命令 | 核心功能 |
|---|---|---|
| **觀測 (Observe)** | `snapshot`, `find`, `get`, `is`, `screenshot`, `list-surfaces` | 取得無障礙樹結構、屬性查詢、元素狀態檢驗、視窗截圖 |
| **Ref 操作 (Act on a ref)** | `click`, `double-click`, `triple-click`, `right-click`, `type`, `set-value`, `clear`, `focus`, `select`, `toggle`, `check`, `uncheck`, `expand`, `collapse`, `scroll`, `scroll-to` | 針對 Ref 執行點擊、文字輸入、值設定、下拉展開、核取方塊切換、捲動等 |
| **鍵盤與滑鼠** | `press`, `hover`, `drag`, `mouse-move`, `mouse-click`, `mouse-wheel`, `key-down`, `key-up`, `mouse-down`, `mouse-up` | 實體按鍵組合（如 `press 'Control+S'`）、滑鼠懸停、座標點擊與滾輪模擬 |
| **視窗與應用管理** | `launch`, `close-app`, `list-apps`, `list-windows`, `list-displays`, `focus-window`, `resize-window`, `move-window`, `minimize`, `maximize`, `restore`, `open-system-surface` | 啟動應用、列出視窗與螢幕、視窗聚焦、縮放、最大化/最小化、開啟系統匣 |
| **通知系統** | `list-notifications`, `dismiss-notification`, `dismiss-all-notifications`, `notification-action` | 讀取系統通知中心、清除通知、觸發通知動作 |
| **系統剪貼簿** | `clipboard-get`, `clipboard-set`, `clipboard-clear` | 安全讀取、寫入與清空系統剪貼簿內容 |
| **工作階段與追蹤** | `wait`, `session`, `trace`, `cursor-overlay` | 延遲等待、追蹤錄製、除錯指標疊加層 |
| **系統與環境** | `status`, `permissions`, `version`, `batch`, `skills` | 權限指引、版本資訊、批次執行封裝、內建 Skill 指令匯出 |

---

### MCP 伺服器與 stdin 輸入（上游 v1.2.0 起）

`agent-desktop mcp` 以 stdio 把每個命令公開成 MCP 工具，工具名稱為 `desktop_` 加命令名（`desktop_snapshot`、`desktop_click`），參數與回傳同該命令的 JSON Envelope：

```json
{ "mcpServers": { "agent-desktop": { "command": "agent-desktop", "args": ["mcp"] } } }
```

`type`、`set-value`、`clipboard-set` 可加 `--stdin` 從標準輸入讀文字，避免機密出現在行程參數；批次 JSON 內的文字會出現在行程參數，機密步驟請分開執行。

MCP client 等同簽入使用者本人，**不是沙盒**。本 fork 預設關閉會執行任意程式、寫檔或刪除的工具，分別用 `agent-desktop mcp --allow-launch`（`launch`）、`--allow-batch`（`batch`）、`--allow-file-write`（檔案輸出；路徑限制在 `<session 目錄>/mcp-output`，啟動時建立，需要 `--session`）、`--allow-session-gc`（`session gc`，會刪除狀態目錄下已結束或過期的 session，範圍不只 MCP 的輸出）、`--allow-cdp`（DevTools／inspector 啟動）開啟。`press`、`key-down`、`key-up`、`close-app` 沒有 `force`，所以危險組合封鎖（`win+r`、`alt+f4` 等）在 batch 內也一律生效，MCP 也不能強制終止程序；MCP 的 `launch` 沒有 `cwd`，也不接受 `PATH`、`PSModulePath`、`PYTHONPATH`、`CLASSPATH`、`DOTNET_STARTUP_HOOKS` 這類決定載入哪個程式或模組、或指定會被執行的輔助程式的環境變數（另含 `RUBYOPT`、`PERL5OPT`、`PYTHONHOME`、`BASH_ENV`、`GIT_SSH_COMMAND`、`GIT_CONFIG_*`，以及 .NET／CoreCLR 的 profiler 設定 `COR_*`、`CORECLR_*`、`DOTNET_*` 中含 `PROFIL` 者），且環境變數名稱必須是 ASCII。這些清單與 DevTools／inspector／JDWP／Firefox `start-debugger-server` 標記都是縱深防禦，不是邊界。**`--allow-launch` 開啟後，client 可以啟動 `cmd` 或 `powershell`，所以 `--allow-cdp` 只擋直接的 DevTools 與 inspector 參數和環境變數，不是獨立的硬邊界。** Windows 的 `launch` 在所有模式都拒絕網路（UNC）與裝置路徑。Windows CLI 的剪貼簿檔案、游標圖片、截圖與 trace 路徑（含全域選項及 batch 項目）在存取檔案前拒絕 UNC 與裝置命名空間；仍接受本機相對路徑與磁碟路徑。理由見 [docs/fork/DECISIONS.md](docs/fork/DECISIONS.md) ADR-011。

---

## 🔌 語言綁定（Language Bindings / FFI）

各平台版本皆包含 C 函式庫 `libagent_desktop_ffi`（Windows 為 `agent_desktop_ffi.dll`），允許外部程式直接透過行程內記憶體呼叫，無需頻繁啟動 CLI 子程序：

```python
import ctypes
lib = ctypes.CDLL("./lib/libagent_desktop_ffi.dylib")
lib.ad_adapter_create.restype = ctypes.c_void_p
lib.ad_adapter_destroy.argtypes = [ctypes.c_void_p]
assert lib.ad_init(7) == 0  # AD_RESULT_OK when the header's AD_ABI_VERSION_MAJOR matches the library
adapter = lib.ad_adapter_create()
lib.ad_adapter_destroy(adapter)
```

---

---

## 🤖 全系統 Agent 整合（SSOT）

本 Fork 成果已作為標準能力發布至 [`SanHsien/agent-assets`](https://github.com/SanHsien/agent-assets) 與全域技能庫，支援各大主力 Agent：

### 支援之 Agent
1. **Google Antigravity**：自動加載全域 Skill（`~/.gemini/config/skills/agent-desktop/SKILL.md`）。
2. **Claude Code / Claude Desktop**：透過通用規範呼叫 `agent-desktop` CLI。
3. **OpenAI Codex**：支援在沙箱或本機以 CLI 驅動。
4. **Hermes**：原生調用命令集。

### 錯誤處理與安全重試合約
所有命令回傳皆包含確定性的 `ok` 與錯誤合約：
- 若元素因畫面重繪或分頁切換而失效，回傳 `code: "STALE_REF"` 與 `disposition.retry: "safe"`。
- Agent 處理合約：**遇到 `STALE_REF` 嚴禁猜測座標，必須重新呼叫 `snapshot` 取得新 Ref 後重試一次**。

---

## 🛠️ 開發與維護

本倉庫維護遵守嚴格的本地與遠端治理門禁：
- **專屬治理規則**：見 [FORK.md](FORK.md)。嚴禁向 upstream 開立 PR 或推送改動。
- **本地品質檢驗**：
  ```powershell
  pwsh -NoProfile -File tools/dev_check.ps1 -Full
  ```
- **單元與衛生測試**：
  ```powershell
  python -m unittest discover -s tests/fork -v
  ```

---

## 📚 延伸文件

- [架構規格手冊（Spec）](docs/agent-desktop-computer-use-spec.md) - 無障礙樹、Ref 算法與安全合約深度技術剖析
- [JSON Envelope 輸出規格](docs/json-output.md) - 資料結構、錯誤代碼與 Exit Code 規範
- [安裝與平台權限指南](docs/install.md) - Windows UIA、macOS 輔助功能權限與 Linux AT-SPI 配置
- [Fork 維護與同步手冊](FORK.md) - Fork 分支治理、CI 與上游追蹤原則
- [概念解析手冊](CONCEPTS.md) - Ref、Snapshot 與 Session 術語集

---

## 📄 授權條款

Apache-2.0 License。詳見 [LICENSE](LICENSE) 與 [NOTICE.md](NOTICE.md)。
