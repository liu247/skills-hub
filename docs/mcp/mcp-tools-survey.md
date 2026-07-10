# Skills Hub · 各 Agent 工具 MCP 配置调研

**分支:** `mcp-test`
**目的:** 为 Skills Hub 后续接入 "MCP 中央管理 + 跨工具同步" 提供权威、可执行的调研基础。所有条目均来源于各工具官方文档,并附原文链接。

---

## 1. 引言与范围

Skills Hub 目前已在 skill 维度实现"一处安装、多处同步"。本调研评估把该模型扩展到 **MCP servers**(Model Context Protocol)的技术前提,重点回答三个问题:

1. **每个宿主工具的 MCP 配置存在哪里、格式是什么、schema 怎样?**
2. **哪些工具的配置格式可以共用一套 adapter?哪些必须单独处理?**
3. **对 Skills Hub 的写入策略(格式保留 / 合并 / ownership marker)有什么约束?**

调研覆盖 Skills Hub 现有 tool_adapters 里所有已知支持 MCP 的宿主,加上桌面端的 Claude Desktop(虽然不在当前 adapter 列表,但是"跨端同步"的核心目标之一)。

---

## 2. MCP 基础术语速览

Model Context Protocol 是 Anthropic 提出的开放协议(https://modelcontextprotocol.io),把外部工具/资源接入 AI agent。三个核心概念:

- **Server**:提供工具/资源的进程,可能是本地 stdio 子进程,也可能是远端 HTTP/SSE 端点
- **Transport**:通信通道
  - `stdio` — 本地进程,通过 stdin/stdout 通信(最常见)
  - `sse` — Server-Sent Events(逐步被弃用,新协议不推荐)
  - `streamable-http` / `http` — 可流式 HTTP(推荐的远程 transport)
  - `ws` — WebSocket(少数工具支持,如 Claude Code)
- **Capabilities**:tools、prompts、resources、roots、elicitation

各家宿主的配置差异,几乎全部集中在**如何在配置文件里描述一个 server**——命令、参数、环境变量、transport、认证、以及一些各家特有的开关(是否禁用、是否自动批准某个工具、超时等等)。

---

## 3. 兼容性总览矩阵

按"配置格式亲缘度"分组,同组内的 schema 高度接近,可以共用一套 adapter。

### 3.1 主流宿主一览表

| 宿主 | 配置文件(全局) | 配置文件(项目) | 格式 | 顶层 key | 备注 |
|---|---|---|---|---|---|
| **Claude Code** | `~/.claude.json`(局部 & 用户) | `.mcp.json`(团队共享) / `.claude/settings.local.json`(个人) | JSON | `mcpServers` | 局部 scope 存在于 `projects["<path>"].mcpServers` 嵌套结构 |
| **Claude Desktop** | `~/Library/Application Support/Claude/claude_desktop_config.json`(mac);`%APPDATA%\Claude\claude_desktop_config.json`(win) | 无 | JSON | `mcpServers` | 最简单、最标准的形态 |
| **Codex (OpenAI)** | `~/.codex/config.toml` | `.codex/config.toml`(仅信任目录) | **TOML** | `[mcp_servers.<name>]` | 唯一使用 TOML 的主流宿主 |
| **Cursor** | `~/.cursor/mcp.json` | `.cursor/mcp.json` | JSON | `mcpServers` | 支持 `${env:VAR}`、`${workspaceFolder}` 等占位符 |
| **Kiro (IDE + CLI)** | `~/.kiro/settings/mcp.json` | `.kiro/settings/mcp.json` | JSON | `mcpServers` | 独有 `autoApprove`、`disabledTools` |
| **Gemini CLI** | `~/.gemini/settings.json` | `.gemini/settings.json` | JSON | `mcpServers`(嵌在 settings 里) | 有独立 `mcp.allowed` / `mcp.excluded` 全局开关 |
| **Qwen Code** | `~/.qwen/settings.json` | `.qwen/settings.json` | JSON | `mcpServers` | Gemini CLI 派生,schema 完全一致 |
| **iFlow CLI** | `~/.iflow/settings.json` | `.iflow/settings.json` | JSON | `mcpServers` | Gemini CLI 派生,schema 完全一致 |
| **Windsurf (Cascade)** | `~/.codeium/windsurf/mcp_config.json` | 无 | JSON | `mcpServers` | 只有全局作用域 |
| **Cline** | `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json` | 无 | JSON | `mcpServers` | VS Code 扩展的 globalStorage |
| **Roo Code** | `~/Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/cline_mcp_settings.json` | 无 | JSON | `mcpServers` | 复用 Cline 的文件名 |
| **Continue** | `~/.continue/config.yaml`(或每个项目 `.continue/mcpServers/*.yaml`) | `.continue/mcpServers/` 目录多文件 | **YAML** | `mcpServers`(数组) | 唯一使用 YAML 的宿主 |
| **Trae / Trae CN** | `~/.trae/mcp.json` | `.trae/mcp.json` | JSON | `mcpServers`(数组) | Schema 变异:数组而非对象 |
| **VS Code (Copilot)** | `<user settings.json>.mcp.servers` 或 `~/Library/Application Support/Code/User/mcp.json` | `.vscode/mcp.json` | JSON | `servers` **(注意不是 mcpServers)** | 顶层 key 是 `servers`,不是 `mcpServers` |
| **Copilot CLI** | `~/.copilot/mcp-config.json` | 无 | JSON | `mcpServers` | 与 VS Code Copilot 有意不统一 |

### 3.2 按 adapter 共享度分簇

从写入实现的复杂度看,可以按下面 4 簇管理:

**簇 A —— "标准 JSON + mcpServers 对象"(最容易统一)**
- Claude Desktop
- Cursor
- Kiro
- Windsurf
- Cline
- Roo Code
- Copilot CLI

共同点:配置文件只/主要装 MCP,顶层 `mcpServers` 是对象,server 名字作为 key。写入时读取 JSON → 修改 `mcpServers[name]` → 保序回写即可。**一套 adapter 覆盖 7 个宿主**。

**簇 B —— "嵌在 settings.json 里的 mcpServers"(需要保留同文件其他字段)**
- Gemini CLI / Qwen Code / iFlow CLI
- Claude Code(user scope 在 `~/.claude.json`,局部 scope 更复杂)

同一个文件里除 MCP 还有其他配置,写入必须**只碰 `mcpServers`(或 `projects.X.mcpServers`)节点**,其他保留。用 `serde_json` 的 `Value` 结构就能做到。**一套 adapter,分两个"落点定位器"**。

**簇 C —— "TOML,需要保序保注释"(单独一套)**
- Codex

Codex 是唯一 TOML 派。用 Rust 的 `toml_edit` 而非 `toml` crate,可以在保留注释和顺序的前提下修改 `[mcp_servers.*]` 段。

**簇 D —— "非标准 schema"(单独处理)**
- **VS Code Copilot**:顶层 key 是 `servers` 不是 `mcpServers`,字段名 `url` vs `httpUrl` 也略有差异
- **Continue**:YAML,`mcpServers` 是数组;字段含 `type` 且优先 `type: sse` / `type: streamable-http`
- **Trae**:JSON 数组格式,每个 server 带 `name` 字段而不是用 key

三个各自都要专属 adapter。

---

## 4. 逐工具详解

### 4.1 Claude Code

**官方文档:** https://docs.claude.com/en/docs/claude-code/mcp

**Scopes(优先级从高到低):**

| Scope | 存储位置 | 共享 |
|---|---|---|
| Local(默认) | `~/.claude.json` 的 `projects["<project_path>"].mcpServers` | 仅当前项目、仅本人 |
| Project | `<project>/.mcp.json` | 提交仓库,团队共享 |
| User | `~/.claude.json` 的顶层 `mcpServers` | 全部项目,仅本人 |
| Plugin | 插件目录内的 `.mcp.json` 或 `plugin.json` | 由插件分发 |

同一 server 出现在多个 scope 时,**取高优先级来源的整条 entry**,不做字段级 merge。

**顶层结构:**

```json
{
  "mcpServers": {
    "<server_name>": { /* server config */ }
  }
}
```

**支持的 transport 与字段:**

- `stdio`(默认,可省略 `type`)
  ```json
  {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-github"],
    "env": {"GITHUB_TOKEN": "ghp_..."},
    "timeout": 600000,
    "alwaysLoad": false
  }
  ```
- `http` / `streamable-http`(推荐远程)
  ```json
  {
    "type": "http",
    "url": "https://mcp.example.com/mcp",
    "headers": {"Authorization": "Bearer ${API_KEY}"},
    "headersHelper": "/opt/bin/get-auth.sh",
    "oauth": {"authServerMetadataUrl": "...", "scopes": "read write"}
  }
  ```
- `sse`(已弃用,但仍可用)—— 同 http,`type: "sse"`
- `ws`(WebSocket)—— 同 http,`type: "ws"`

**特殊字段:**
- `alwaysLoad`(bool)—— 该 server 的 tool 是否总是加载(不走 lazy tool search)
- `timeout`(ms)—— 单 tool 调用超时
- `headersHelper`(命令)—— 动态生成认证头
- `oauth`(对象)—— 覆盖 OAuth metadata 发现或限定 scopes

**环境变量占位符:**
支持 `${VAR}` 和 `${VAR:-default}`,可出现在 `command`、`args`、`env`、`url`、`headers` 里。

**CLI 管理命令:**
```
claude mcp add <name> [options] -- <command> [args...]
claude mcp add --transport http <name> <url>
claude mcp add-json <name> '<json>'
claude mcp list
claude mcp remove <name>
claude mcp add-from-claude-desktop     # 从 Claude Desktop 导入
```

**重载机制:**
- 项目级 `.mcp.json` 需要用户在会话里明确批准(prompt)
- HTTP/SSE 自动重连,stdio 不重连
- 支持 `tools/list_changed` 通知的服务器可以动态更新工具集

**注意事项:**
- Local scope 的存储方式(嵌套在 `projects.<path>` 下)有 bug:子目录不会继承父目录的 servers。写入时要谨慎处理路径规范化。
- `workspace` 是保留 server 名,不能用。
- 服务器名不建议含下划线(Gemini/Trae 也一样),后续做 permission 匹配会踩坑。

---

### 4.2 Claude Desktop

**官方文档:** https://modelcontextprotocol.io/docs/develop/connect-local-servers

**配置文件:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`(社区路径)

**Schema:** 最简单,只有 `mcpServers` 一个顶层 key,只支持 stdio:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"],
      "env": {}
    }
  }
}
```

**特殊字段:** 无。极简。

**重载机制:** 需**重启 Claude Desktop**。修改后不自动刷新。

**日志:** `~/Library/Logs/Claude/mcp.log` 和 `mcp-server-<name>.log`

---

### 4.3 OpenAI Codex

**官方文档:** https://developers.openai.com/codex/mcp

**配置文件:**
- 全局: `~/.codex/config.toml`
- 项目: `.codex/config.toml`(仅信任目录)

**格式:** **TOML**。每个 server 用 `[mcp_servers.<name>]` 表描述。

**支持的 transport 与字段:**

**stdio:**
```toml
[mcp_servers.context7]
command = "npx"                              # 必填
args = ["-y", "@upstash/context7-mcp"]       # 可选
cwd = "/tmp/context7"                        # 可选,工作目录
env_vars = ["LOCAL_TOKEN"]                   # 从本地环境转发的变量
experimental_environment = "remote"          # 可选,通过远程执行器起动

[mcp_servers.context7.env]                   # 硬编码环境变量
MY_ENV_VAR = "hello"
```

**Streamable HTTP:**
```toml
[mcp_servers.figma]
url = "https://mcp.figma.com/mcp"
bearer_token_env_var = "FIGMA_OAUTH_TOKEN"   # 认证 header 取自环境变量
http_headers = { "X-Figma-Region" = "us-east-1" }  # 静态 headers
env_http_headers = { "X-User" = "USER_NAME" }       # 从环境变量取值的 headers
```

**通用可选字段:**
- `startup_timeout_sec`(默认 10)
- `tool_timeout_sec`(默认 60)
- `enabled`(bool,默认 true)—— 停用不删
- `required`(bool)—— 启动时该 server 若无法初始化则整个启动失败
- `enabled_tools` / `disabled_tools`(数组)—— 工具白/黑名单
- `default_tools_approval_mode`—— `auto` / `prompt` / `approve`
- `tools.<tool>.approval_mode`—— 每工具审批模式覆盖

**顶层 OAuth 设置:**
```toml
mcp_oauth_callback_port = 5555
mcp_oauth_callback_url = "https://devbox.example/callback"
```

**插件提供的 server:**
```toml
[plugins."sample@test".mcp_servers.sample]
enabled = true
default_tools_approval_mode = "prompt"
```
用户配置只能开关,不能改 command/url(由插件宿主)。

**CLI:** `codex mcp add`、`codex mcp login`、`codex mcp` TUI(`/mcp` 命令查看)

**重载机制:** 修改 `config.toml` 后**重启 Codex**。

**写入难点:**
- 保留注释/顺序需要 `toml_edit`(不是 `toml`)
- Codex 的 `config.toml` 同时含很多其他配置(model、approvals、network、tokens),不能整体覆写

---

### 4.4 Cursor

**官方文档:** https://cursor.com/docs/context/mcp

**配置文件:**
- 全局: `~/.cursor/mcp.json`
- 项目: `.cursor/mcp.json`

**Schema:**

**stdio:**
```json
{
  "mcpServers": {
    "server-name": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "mcp-server"],
      "env": {"API_KEY": "${env:API_KEY}"},
      "envFile": "${workspaceFolder}/.env"
    }
  }
}
```

**远程(HTTP/SSE):**
```json
{
  "mcpServers": {
    "oauth-server": {
      "url": "https://api.example.com/mcp",
      "headers": {"Authorization": "Bearer ${env:MY_TOKEN}"},
      "auth": {
        "CLIENT_ID": "your-oauth-client-id",
        "CLIENT_SECRET": "your-client-secret",
        "scopes": ["read", "write"]
      }
    }
  }
}
```

**特殊字段:**
- `envFile`(stdio 独有)—— 从 `.env` 文件加载额外变量,支持 `${workspaceFolder}` 占位
- `auth.CLIENT_ID/CLIENT_SECRET/scopes`—— 静态 OAuth 客户端凭证

**占位符系统:**
- `${env:NAME}`—— 环境变量
- `${userHome}`—— 家目录
- `${workspaceFolder}` / `${workspaceFolderBasename}`—— 项目根
- `${pathSeparator}` / `${/}`—— 路径分隔符

生效字段:`command`, `args`, `env`, `url`, `headers`, `auth`

**重载机制:** Cursor 自动检测 `mcp.json` 变化并重连;也可以在 Customize 面板里显式开关。

---

### 4.5 Kiro (IDE + CLI)

**官方文档:**
- IDE: https://kiro.dev/docs/mcp/configuration
- CLI: https://kiro.dev/docs/cli/mcp/configuration

**配置文件:**
- 用户级: `~/.kiro/settings/mcp.json`
- 工作区级: `.kiro/settings/mcp.json`
- Agent 级(仅 CLI): 每个 agent JSON 里的 `mcpServers` 字段

三级同时生效,**合并**(不覆盖)。

**Schema:**

**本地 server:**
```json
{
  "mcpServers": {
    "local-server-name": {
      "command": "command-to-run-server",
      "args": ["arg1", "arg2"],
      "env": {
        "ENV_VAR1": "hard-coded-variable",
        "ENV_VAR2": "${EXPANDED_VARIABLE}"
      },
      "disabled": false,
      "autoApprove": ["tool_name1", "tool_name2"],
      "disabledTools": ["tool_name3"]
    }
  }
}
```

**远程 server:**
```json
{
  "remote-server-name": {
    "url": "https://mcp.example.com/mcp",
    "headers": {"HEADER1": "value1"},
    "oauth": {"clientId": "your-app-client-id"},
    "oauthScopes": ["scope1", "scope2"],
    "disabled": false,
    "autoApprove": ["tool_name1"],
    "disabledTools": ["tool_name3"]
  }
}
```

**Kiro 独有字段:**
- `disabled`(bool)—— 停用不删
- `autoApprove`(string[])—— 无需人工确认就可运行的工具名列表;`"*"` 代表全部
- `disabledTools`(string[])—— 从工具目录中隐藏
- `oauthScopes`(string[])

**重载机制:** Kiro IDE 会**监听 `~/.kiro/settings/mcp.json`(全局)和 `.kiro/settings/sandbox.json`**,变更 300ms 防抖后自动重扫。

---

### 4.6 Gemini CLI(及派生:Qwen Code、iFlow CLI)

**官方文档:** https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md

**配置文件:**
- 全局: `~/.gemini/settings.json`
- 项目: `.gemini/settings.json`

**Qwen Code** 和 **iFlow CLI** 是 Gemini CLI 的 fork,配置格式完全一致,只是路径分别是 `~/.qwen/settings.json` 和 `~/.iflow/settings.json`。**一套 adapter 覆盖 3 个宿主**。

**顶层结构:** MCP 相关有两块:
```json
{
  "mcp": {
    "allowed": ["my-trusted-server"],
    "excluded": ["experimental-server"],
    "serverCommand": "..."
  },
  "mcpServers": {
    "<name>": { /* server config */ }
  }
}
```

**Schema(每个 server):**

必填(三选一):
- `command`(stdio)
- `url`(SSE)—— 例:`"http://localhost:8080/sse"`
- `httpUrl`(Streamable HTTP)

可选:
- `args`(string[])
- `headers`(object,用于 url/httpUrl)
- `env`(object,支持 `$VAR` / `${VAR}` / `%VAR%`)
- `cwd`(string)
- `timeout`(ms,默认 600000)
- `trust`(bool)—— true 时跳过所有确认
- `includeTools`(string[])—— 白名单
- `excludeTools`(string[])—— 黑名单,**优先于** includeTools
- `authProviderType`—— `dynamic_discovery`(默认)/`google_credentials` / `service_account_impersonation`
- `targetAudience`, `targetServiceAccount`—— GCP 相关
- `oauth`(对象,`clientId`、`scopes`、`redirectUri` 等)

**环境变量沙箱:** 默认自动删除敏感变量(`*TOKEN*`、`*SECRET*` 等)不传给 server,除非在 `env` 里显式列出。

**扩展合并规则:** 当 MCP server 来自扩展时,用户本地 `settings.json` 里的同名 server 与扩展合并:
- `env` 用户覆盖扩展
- `excludeTools` 求并集(更严格)
- `includeTools` 求交集(更严格)
- `command`/`url`/`timeout` 等标量用户覆盖

**CLI:**
```
gemini mcp add [--scope user|project] [--transport stdio|http|sse] <name> <cmd|url> [args...]
gemini mcp list
gemini mcp remove <name>
gemini mcp enable/disable <name> [--session]
```

**重载机制:** 修改 `settings.json` 后新启动的会话生效;运行时可用 `/mcp enable <name>` 临时切换(存 `~/.gemini/mcp-server-enablement.json`)。

---

### 4.7 Windsurf (Cascade)

**官方文档:** https://docs.devin.ai/desktop/cascade/mcp

**配置文件:**
- macOS/Linux: `~/.codeium/windsurf/mcp_config.json`
- Windows: `%USERPROFILE%\.codeium\windsurf\mcp_config.json`

**只有全局作用域**,无项目级。

**Schema:** 标准 `mcpServers` 对象:

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {"GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_..."}
    },
    "remote-example": {
      "serverUrl": "https://mcp.example.com/mcp",
      "headers": {"Authorization": "Bearer ${env:MY_TOKEN}"}
    }
  }
}
```

**注意:** Windsurf 用 `serverUrl`(不是 `url`)描述远程端点。

**占位符系统:**
- `${env:VAR_NAME}`—— 环境变量
- `${file:/path/to/file}`—— 文件内容(去空白)
- 生效字段:`command`, `args`, `env`, `serverUrl`, `url`, `headers`

**注意事项:**
- 只支持 stdio + streamable HTTP + SSE
- 100 tool 上限;20 tool-call per prompt 上限
- 修改后需重启 Windsurf

---

### 4.8 Cline(VS Code 扩展)

**官方文档:** https://docs.cline.bot/mcp/mcp-overview

**配置文件:**
- macOS: `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json`
- Windows: `%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json`
- Linux: `~/.config/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json`
- Cline CLI: `~/.cline/mcp.json`

**Schema:** 标准 `mcpServers` 对象,支持 stdio + streamable HTTP + SSE(legacy)。

```json
{
  "mcpServers": {
    "local-server": {
      "command": "node",
      "args": ["server.js"],
      "env": {"API_KEY": "value"},
      "disabled": false,
      "alwaysAllow": ["tool1", "tool2"],
      "timeout": 60
    },
    "remote-server": {
      "type": "streamableHttp",
      "url": "https://mcp.example.com/mcp",
      "headers": {"X-API-Key": "..."},
      "alwaysAllow": ["tool_a"],
      "disabled": false
    }
  }
}
```

**Cline 独有字段:**
- `disabled`(bool)
- `alwaysAllow`(string[])—— 类似 Kiro 的 `autoApprove`
- `type`—— 默认 `sse`(向后兼容),推荐显式 `"streamableHttp"`

**重载机制:** 在 Cline 面板可 enable/disable/restart/remove;修改 JSON 后扩展会自动 reload。

---

### 4.9 Roo Code(VS Code 扩展)

**官方文档:** https://roocodeinc.github.io/Roo-Code/features/mcp/using-mcp-in-roo

**配置文件:**
- macOS: `~/Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/cline_mcp_settings.json`
- 其他 OS 结构相同,只是 `globalStorage/rooveterinaryinc.roo-cline/` 路径不同

**Schema:** 与 Cline 完全一致(fork 自 Cline),字段兼容。

```json
{
  "mcpServers": {
    "server1": {
      "command": "python",
      "args": ["/path/to/server.py"],
      "env": {"API_KEY": "..."},
      "alwaysAllow": ["tool1", "tool2"],
      "disabled": false
    },
    "modern-remote-server": {
      "type": "streamable-http",
      "url": "https://mcp.example.com/mcp",
      "headers": {"X-API-Key": "..."},
      "alwaysAllow": ["newToolA"],
      "disabled": false
    }
  }
}
```

**类型标记差异:** Roo Code 用连字符 `"streamable-http"`,Cline 用驼峰 `"streamableHttp"`。写入时要注意这一处细节。

---

### 4.10 Continue

**官方文档:** https://docs.continue.dev/customize/deep-dives/mcp

**配置文件(YAML):**
- 用户级: `~/.continue/config.yaml`(或 `config.json`,旧版)
- 项目级: `.continue/mcpServers/*.yaml`(**每个 server 一个文件**)

**Schema(YAML):**

```yaml
name: Playwright mcpServer
version: 0.0.1
schema: v1
mcpServers:
  - name: Browser Search
    command: npx
    args:
      - "@playwright/mcp@latest"
  - name: Remote SSE Server
    type: sse
    url: https://mcp.example.com/sse
  - name: Secure MCP Server
    command: npx
    args: ["-y", "@supabase/mcp-server-supabase@latest"]
    env:
      SUPABASE_TOKEN: ${{ secrets.SUPABASE_TOKEN }}
```

**关键差异:**
- **YAML 格式**(唯一)
- **`mcpServers` 是数组**(不是对象),每个 entry 有独立的 `name` 字段
- 支持 `${{ secrets.NAME }}` 引用 Continue 的密钥仓库
- Continue 会把 Claude/Cursor/Cline 格式的 JSON 文件放到 `.continue/mcpServers/*.json` 里也能识别

**Transport:**
- 默认 stdio(有 `command` 即 stdio)
- `type: sse` 或 `type: streamable-http` 显式指定远程

**注意事项:**
- 只在 agent mode 生效
- 官方推荐 YAML,但也兼容 JSON(直接从其他工具拷贝)

---

### 4.11 Trae / Trae CN

**官方文档:** https://traeide.com/news/6

**配置文件:**
- 全局: `~/.trae/mcp.json`
- 项目: `.trae/mcp.json`

**Schema(JSON,但 `mcpServers` 是数组):**

```json
{
  "mcpServers": [
    {
      "name": "supabase_local",
      "command": ["supabase", "mcp"],
      "env": {"SUPABASE_ACCESS_TOKEN": "YOUR_TOKEN"}
    },
    {
      "name": "github_agent",
      "type": "sse",
      "url": "https://mcp.github.com/agent"
    }
  ]
}
```

**关键差异:**
- `mcpServers` **数组**而非对象
- `command` 可以是字符串数组(`["supabase", "mcp"]`)—— 与其他工具的"命令字符串 + args 数组"不同

**Trae 也支持** `.rules` 文件:`.trae/project_rules.md` 和 `.trae/user_rules.md`(与 MCP 无关)。

---

### 4.12 VS Code(GitHub Copilot)

**官方文档:** https://docs.github.com/en/copilot/how-tos/provide-context/use-mcp-in-your-ide/set-up-the-github-mcp-server

**配置文件:**
- 项目: `.vscode/mcp.json`(直接就是 `servers` 对象)
- 用户: `settings.json` 里嵌套 `mcp.servers` 对象,或独立文件 `~/Library/Application Support/Code/User/mcp.json`

**Schema:** **注意顶层 key 是 `servers`,不是 `mcpServers`!**

```json
{
  "servers": {
    "github": {
      "type": "http",
      "url": "https://api.githubcopilot.com/mcp/",
      "headers": {"Authorization": "Bearer YOUR_GITHUB_PAT"}
    }
  }
}
```

**在 user settings.json 里:**
```json
{
  "mcp": {
    "servers": {
      "github": { /* ... */ }
    }
  }
}
```

**关键差异:**
- 顶层 key 名不同(`servers`)
- 权限授权分三级("Allow in this Session"/"Allow in this Workspace"/"Always Allow"),存储在 VS Code 的设置里

---

### 4.13 Copilot CLI

**参考:** GitHub Community 讨论 https://github.com/orgs/community/discussions/187954

**配置文件:**
- 全局: `~/.copilot/mcp-config.json`

**Schema:** 与 Claude Desktop 几乎一致,顶层 `mcpServers` 对象。

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {"GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_..."}
    }
  }
}
```

**注意:** VS Code Copilot 用 `servers`,Copilot CLI 用 `mcpServers` —— 同一家产品线里两处不统一,已经有 GitHub issue 请求收敛。

---

## 5. 横向对比与共性

### 5.1 字段名对照表

按语义分类,不同工具用了不同名字。写 adapter 时这张表就是"翻译词典"。

| 语义 | 常见名 | 变体 |
|---|---|---|
| 顶层 servers 集合 | `mcpServers`(多数) | `servers`(VS Code Copilot) |
| server 集合形态 | object(多数) | array(Continue、Trae) |
| stdio 命令 | `command`(string) | `command`(array,Trae) |
| stdio 参数 | `args` | 无变体 |
| 环境变量 | `env` | Codex 用 `env`(硬编码)+ `env_vars`(转发) |
| 工作目录 | `cwd` | Codex `cwd`,其他多不支持 |
| 远程 URL | `url` | `httpUrl`(Gemini)、`serverUrl`(Windsurf) |
| HTTP headers | `headers` | Codex 用 `http_headers` + `env_http_headers` |
| Transport 显式声明 | `type` | Codex 依据是否有 `url` 判定;Claude Code 用 `type` |
| 停用 | `disabled` | Codex `enabled: false`;VS Code 通过 UI 授权 |
| 工具白名单 | `enabled_tools`(Codex) | `includeTools`(Gemini) |
| 工具黑名单 | `disabled_tools`(Codex) | `excludeTools`(Gemini)、`disabledTools`(Kiro) |
| 自动批准 | `autoApprove`(Kiro) | `alwaysAllow`(Cline/Roo)、`default_tools_approval_mode`(Codex)、`trust`(Gemini) |
| 超时(启动) | `startup_timeout_sec`(Codex) | 其他无独立字段 |
| 超时(调用) | `tool_timeout_sec`(Codex) | `timeout`(Claude Code/Gemini/Cline,ms) |

### 5.2 Transport 支持矩阵

| 宿主 | stdio | SSE | streamable HTTP | WebSocket |
|---|:-:|:-:|:-:|:-:|
| Claude Code | ✅ | ✅(废弃) | ✅ | ✅ |
| Claude Desktop | ✅ | ❌ | ❌ | ❌ |
| Codex | ✅ | ❌ | ✅ | ❌ |
| Cursor | ✅ | ✅ | ✅ | ❌ |
| Kiro | ✅ | ✅ | ✅ | ❌ |
| Gemini CLI(+ Qwen/iFlow) | ✅ | ✅ | ✅(`httpUrl`) | ❌ |
| Windsurf | ✅ | ✅ | ✅ | ❌ |
| Cline / Roo Code | ✅ | ✅ | ✅ | ❌ |
| Continue | ✅ | ✅ | ✅ | ❌ |
| Trae | ✅ | ✅ | ❌(可能) | ❌ |
| VS Code Copilot | ✅ | ✅ | ✅ | ❌ |

**结论:** stdio 是最大公约数,streamable HTTP 是次大公约数。SSE 到处支持但正被弃用。WebSocket 只有 Claude Code 一家。

Skills Hub 若做同步,建议**先只支持 stdio + streamable HTTP**,覆盖 95% 场景。

### 5.3 环境变量占位符

| 宿主 | 占位符语法 |
|---|---|
| Claude Code | `${VAR}` / `${VAR:-default}` |
| Cursor | `${env:VAR}` / `${userHome}` / `${workspaceFolder}` / `${pathSeparator}` |
| Gemini CLI | `$VAR` / `${VAR}` / `%VAR%`(win) |
| Kiro | `${VAR}` |
| Windsurf | `${env:VAR}` / `${file:/path}` |
| Codex | 无占位符,用 `env_vars` 列白名单转发 + `bearer_token_env_var` 引用 |
| VS Code Copilot | 使用 VS Code 自己的 `${input:xxx}` 变量语法 |
| Continue | `${{ secrets.NAME }}`(自有 secrets 仓库) |

**结论:** **无法用单一占位符语法在所有宿主间同步**。Skills Hub 的中央库要么用"抽象语义 + 每工具翻译",要么强制使用**未展开的环境变量名**(如存 `"env": {"KEY": "$MY_KEY"}` 字面量),让每个宿主自己去解析。后者是更现实的选择。

### 5.4 认证与秘钥

- **API Key/Token**:多数走 `env`,少数(Codex)有专门的 `bearer_token_env_var`
- **OAuth**:Claude Code、Cursor、Gemini CLI、Codex 都支持自动发现 + PKCE 流程;客户端凭据存在系统 keychain
- **动态 Header**:只有 Claude Code 支持 `headersHelper`(每次连接跑一段 shell 生成 headers)

Skills Hub 要跨工具同步 API key 的话,最安全的策略:
1. **不同步秘钥本身**,只同步 `"env": {"KEY_NAME": "$ENV_VAR"}` 这种引用形式
2. 用户在各宿主的 shell 环境里设置一次 `$ENV_VAR`
3. 或者提供"复制到剪贴板"按钮,让用户手动粘到各宿主里

### 5.5 重载机制

| 宿主 | 修改配置后 |
|---|---|
| Claude Code | 新会话生效;已开会话可 `/mcp` 里手动 restart |
| Claude Desktop | **必须重启 app** |
| Codex | **必须重启** |
| Cursor | 自动检测并重连 |
| Kiro | 自动监听文件变化(300ms 防抖) |
| Gemini CLI | 新会话生效;运行时可 `/mcp disable/enable` |
| Windsurf | 需重启 |
| Cline / Roo Code | 扩展会自动 reload |

**结论:** Skills Hub 写入配置后,应在 UI 里明确告诉用户"需要重启 X"或"Y 会自动生效",避免困惑。

---

## 6. 对 Skills Hub 的实施启示

### 6.1 中央库 schema 设计(建议)

数据库表 `mcp_servers`,字段:

```
id                TEXT PRIMARY KEY
name              TEXT NOT NULL         -- server 名(用户可见)
collection        TEXT NULL             -- 复用现有系列机制
transport         TEXT NOT NULL         -- 'stdio' | 'http' | 'sse' | 'ws'
command           TEXT NULL             -- stdio 必填
args              TEXT NULL             -- JSON 数组
env               TEXT NULL             -- JSON 对象;值优先用 $VAR 引用形式
cwd               TEXT NULL
url               TEXT NULL             -- 远程必填
headers           TEXT NULL             -- JSON 对象
timeout_ms        INTEGER NULL
description       TEXT NULL
enabled           INTEGER NOT NULL DEFAULT 1
auto_approve      TEXT NULL             -- JSON string[]
disabled_tools    TEXT NULL             -- JSON string[]
created_at        INTEGER NOT NULL
updated_at        INTEGER NOT NULL
```

表 `mcp_server_targets`(类比 `skill_targets`):

```
id             TEXT PRIMARY KEY
mcp_server_id  TEXT NOT NULL REFERENCES mcp_servers(id)
tool           TEXT NOT NULL
scope          TEXT NOT NULL DEFAULT 'global'    -- global | project
project_path   TEXT NULL
status         TEXT NOT NULL                     -- ok | error | disabled
last_error     TEXT NULL
synced_at      INTEGER NULL
```

### 6.2 Adapter 数量估计

按 §3.2 分簇:

| 簇 | Adapter 数量 | 覆盖宿主数 |
|---|---|---|
| A(标准 JSON + mcpServers 对象) | 1 | 7 |
| B(嵌在大 settings.json 里) | 1 | 4 |
| C(Codex,TOML) | 1 | 1 |
| D(异形:VS Code Copilot、Continue、Trae) | 3 | 3 |
| **合计** | **6 个 adapter** | **15 宿主** |

一个不错的 leverage:约 6 个 adapter 覆盖 Skills Hub 现有大多数活跃工具。

### 6.3 Ownership 与 merge 策略

**核心原则:只碰自己写过的 entry,尊重用户手改。**

实现方式(推荐):

1. **在写入的每个 server entry 里嵌一个 marker 字段**,例如:
   ```json
   {
     "mcpServers": {
       "github": {
         "command": "npx", "args": [...],
         "_managed_by": "skills-hub"
       }
     }
   }
   ```
   TOML 派可以用注释:`# managed_by = "skills-hub"`
   
2. Skills Hub 在数据库里也记录**"这个 server 名+这个宿主"我们上次写入过**(通过 `mcp_server_targets` 表)。
3. 写入前 `git-diff-style` 预览,让用户确认。
4. **绝不删除自己没写过的 entry**。
5. 大配置文件(Codex `config.toml`、Gemini `settings.json`)写入前必须 **backup**(带时间戳)。

### 6.4 分阶段实施建议

按之前和你讨论的三阶段推进,现在有了详细文档后可以更精确定义每一阶段:

**阶段 1 · 只读盘点** (风险 0)
- 6 个 adapter 的**读取**能力:parse 各家配置,产出统一的 `McpServerRecord` 列表
- UI 展示 "server × 宿主" 矩阵
- **推荐第一批实现顺序:** Claude Desktop → Cursor → Kiro → Codex → Claude Code(local scope 最复杂,放最后)

**阶段 2 · 中央库 + 单向写入(Skills Hub → 宿主)**
- 相同 6 个 adapter 的**写入**能力
- Import(从现有宿主导入到中央库)+ Sync(从中央库同步到宿主)
- Ownership marker + backup + diff preview

**阶段 3 · 增强**
- Marketplace 发现(mcp.so、smithery.ai)
- 秘钥 keychain 集成
- 健康检查(spawn 一次 `initialize` + `tools/list`)

### 6.5 需要额外确认的点

- **Trae 的 `command` 数组形式**:官方样例是 `["supabase", "mcp"]`,但实际 IDE 是否也接受 `command: "supabase"` + `args: ["mcp"]`?建议装一份 Trae 实测。
- **VS Code Copilot 的 `mcp.servers` 在 user settings.json 中**:VS Code 会自动同步 user settings 到设置同步服务,如果 Skills Hub 写入这里,会被 Settings Sync 分发到用户其他设备——**这可能是好事也可能是坏事**,取决于用户设备的 shell 环境是否一致。
- **Gemini CLI 敏感变量沙箱**:如果我们写 `"env": {"GITHUB_TOKEN": "$GITHUB_TOKEN"}`,Gemini 会**看到我们显式列出了**,不会去掉。但如果只写 `"env": {}`,Gemini 会阻止 host 端的 `GITHUB_TOKEN` 传给 server。中央库同步时要注意保留必要的 env 字段。

---

## 附录 A · 原始文档链接

- Claude Code MCP: https://docs.claude.com/en/docs/claude-code/mcp
- Claude Desktop MCP(通用): https://modelcontextprotocol.io/docs/develop/connect-local-servers
- OpenAI Codex MCP: https://developers.openai.com/codex/mcp
- Cursor MCP: https://cursor.com/docs/context/mcp
- Kiro IDE MCP: https://kiro.dev/docs/mcp/configuration
- Kiro CLI MCP: https://kiro.dev/docs/cli/mcp/configuration
- Gemini CLI MCP: https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md
- Qwen Code MCP: https://qwenlm.github.io/qwen-code-docs/en/developers/tools/mcp-server
- iFlow CLI MCP: https://platform.iflow.cn/en/cli/examples/mcp
- Windsurf Cascade MCP: https://docs.devin.ai/desktop/cascade/mcp
- Cline MCP: https://docs.cline.bot/mcp/mcp-overview
- Roo Code MCP: https://roocodeinc.github.io/Roo-Code/features/mcp/using-mcp-in-roo
- Continue MCP: https://docs.continue.dev/customize/deep-dives/mcp
- Trae MCP: https://traeide.com/news/6
- VS Code Copilot MCP: https://docs.github.com/en/copilot/how-tos/provide-context/use-mcp-in-your-ide/set-up-the-github-mcp-server
- MCP 协议规范: https://modelcontextprotocol.io

---

## 附录 B · 用户机器现有 MCP 快照(2026-07-09)

用户 `~/.codex/config.toml` 已配置的 servers(通过 `grep '^\[mcp_servers\.' ~/.codex/config.toml` 得到):

- linear
- playwright
- spec-coding-mcp
- tavily
- node_repl
- transfer-ai
- mcp-pdf
- scihub

其他配置文件:
- `~/.kiro/settings/mcp.json` 存在
- `~/.claude/settings.json` 存在,`mcpServers` 为空
- `~/Library/Application Support/Claude/claude_desktop_config.json` 存在,`mcpServers` 为空
- `~/.cursor/mcp.json` **不存在**

这是"典型的 8 个 server 只装在一个宿主里"的场景,验证了 Skills Hub 同步 MCP 的产品价值。

---

*编写:2026-07-09,基于官方文档最新版本。字段行为可能随各宿主升级变化,实施前建议对 §4 中每个宿主的最新文档做一次快速交叉验证。*
