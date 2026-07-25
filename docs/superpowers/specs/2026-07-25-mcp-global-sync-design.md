# MCP 全局同步（Codex、Claude Code、Kiro、Reasonix）设计

## 目标与范围

将一个由 Skills Hub 管理的 MCP Server 同步到 Codex、Claude Code、Kiro 与 Reasonix 四个宿主。第一阶段仅支持用户级（全局）配置；不创建或修改项目级 MCP 配置。

本阶段覆盖 Server 的创建、编辑、目标选择、单向同步、结果展示与错误提示，以及统一的操作系统安全凭据库密钥管理。导入现有配置、双向合并、项目级 scope、Marketplace 与健康检查均不在范围内。

## 核心模型

`McpServer` 是唯一真源，保存四个宿主通用的定义：

- `id`、`name`、`description`、`enabled`
- `transport`：`stdio` 或 `http`
- stdio 字段：`command`、`args`、`env`、`cwd`
- HTTP 字段：`url`、`headers`
- `timeout_ms`（可选）

`McpServerTarget` 记录一个 Server 到一个宿主的同步关系：`mcp_server_id`、`tool`、`scope=global`、`status`、`last_error`、`synced_at`。目标级扩展字段只用于不能无损映射的宿主特性，例如 Codex 的启用/工具审批设置、Kiro 的 `autoApprove` 与 `disabledTools`。

中央模型不保存明文密钥；`env`、HTTP headers 只保存环境变量引用或非敏感静态值。

## 密钥管理

操作系统安全凭据库是唯一的真实密钥存储：macOS 使用 Keychain、Windows 使用 Credential Manager、Linux 使用 Secret Service。应用数据库只保存稳定的密钥引用（服务名与环境变量名），绝不保存密钥值或可逆加密副本；不提供主密码或可移植密钥库作为替代路径。

一个 MCP Server 可关联多个命名密钥，例如 `GITHUB_TOKEN` 和 `STRIPE_KEY`。用户在 Skills Hub 中录入、更新或删除一次密钥后，所有已选宿主在下次同步时均从同一安全凭据库生成其所需的环境变量引用或认证字段，无需在各 App 重复输入。

同步、备份、差异预览、SQLite 数据、日志、Tauri IPC DTO 和错误消息都不得包含明文密钥。若某宿主无法以环境变量或等效的安全引用表达所需认证，adapter 必须阻止该目标同步并给出明确的能力诊断，禁止降级为明文写入配置文件。

从现有宿主配置导入明文密钥并迁移到安全凭据库不属于第一阶段；后续实现时必须先写入凭据库、重新同步目标配置，并由用户明确确认是否移除原配置中的明文。

## 宿主适配器

每个 adapter 负责配置路径解析、读取/合并、从中央模型生成目标格式、备份和重载提示。同步操作只修改 MCP 所在节点，保留同文件其他配置。

| 宿主 | 全局路径 | 写入格式 | 约束 |
| --- | --- | --- | --- |
| Codex | `~/.codex/config.toml` | `[mcp_servers.<name>]` | 使用 `toml_edit` 定点写入，保留注释与顺序；写入前备份；提示重启。 |
| Claude Code | `~/.claude.json` | 顶层 `mcpServers.<name>` | 仅改 User scope；不触及 `projects`、`.mcp.json` 或插件；提示重启。 |
| Kiro | `~/.kiro/settings/mcp.json` | `mcpServers.<name>` | 支持 stdio/HTTP；按需写入 Kiro 专属审批字段；Kiro 自动重载。 |
| Reasonix | `~/.reasonix/config.toml` | `[[plugins]]` | 写入 `type=stdio` 或 `type=http`；不写 `.mcp.json`，避免与 `reasonix.toml` 的同名优先级冲突；提示重启/刷新。 |

不支持的字段不会静默丢弃：同步前 adapter 返回能力诊断，UI 显示被忽略字段及原因，并允许用户取消。

## 写入、所有权与错误处理

1. 用户选择一个 Server 和一个或多个全局目标。
2. 对每个目标读取当前配置，解析后生成差异预览。
3. 目标配置存在时创建带时间戳的同目录备份。
4. 仅创建或更新数据库中已登记的 `McpServerTarget`；从不删除未登记的宿主条目。
5. 原子写入临时文件后替换原文件；单个目标失败不回滚其他已成功目标，结果逐目标记录。
6. 记录成功/失败状态、错误及同步时间，UI 提示宿主是否需要重启或已自动重载。

Server 名称统一校验为跨宿主安全的标识符（小写字母开头，仅含小写字母、数字与连字符），避免 Claude Code 保留名及下划线兼容性问题。

## 用户界面与命令层

新增 MCP 管理入口，提供 MCP 列表、创建/编辑表单、目标选择、同步预览与每目标状态。所有新增文字提供中英文翻译。

Tauri command 层只负责 DTO 与错误转换。MCP 的 SQLite store、模型校验、适配器、备份/原子写入和同步编排放在 Rust core 层，并为每个 adapter 及同步失败场景编写独立测试。

## 验证标准

- 每个宿主能从同一 stdio 或 HTTP Server 定义生成正确的全局配置片段。
- 不相关配置、注释（Codex）和未管理 MCP 条目保持不变。
- 配置写入失败时原文件保持完整，目标状态记录错误。
- 已管理 Server 的重复同步是幂等的。
- 密钥值不出现在 SQLite、配置文件、备份、预览、日志、DTO 或错误信息中。
- `npm run check` 通过。
