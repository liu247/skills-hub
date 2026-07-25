# 统一网络代理

## 功能概述

v0.7.1 将“网络代理”定义为 Skills Hub 的应用级联网代理。用户开启后，应用内发起的外部网络请求统一交给该代理入口，例如 `http://127.0.0.1:7890`。具体哪些站点直连、哪些站点转发，由用户本机代理软件或 VPN 规则决定。

## 背景

v0.7.0 的网络代理主要覆盖 GitHub 相关能力：

- GitHub 搜索
- 精选 Skills 拉取
- GitHub Contents API 下载
- Git clone/fetch/update

但还有几条外部联网路径没有使用同一配置：

- 应用自更新检查。
- 应用自更新下载和安装。
- 更新弹窗中的 release notes 拉取。
- Explore 在线搜索 `skills.sh`。

这会造成用户体验不一致：用户已经在设置里开启了网络代理，但部分联网能力仍可能直连失败。

## 设计原则

- 应用不判断域名是否“应该代理”。
- 开启代理后，应用外部联网请求统一使用同一个代理入口。
- 是否真正转发由本机代理软件或 VPN 的规则决定。
- 关闭代理后，请求使用系统默认网络行为。
- 不把代理规则拆散到每个功能里，避免用户理解成本和维护成本上升。

## 影响范围

### 需要补齐

- `@tauri-apps/plugin-updater` 的 `check`。
- `@tauri-apps/plugin-updater` 的 `downloadAndInstall`。
- 更新弹窗 release notes 拉取。
- `skills.sh` 在线搜索。

### 已覆盖，保持现状

- GitHub API 搜索。
- 精选 Skills 拉取。
- GitHub Contents API 下载。
- Git clone/fetch/update。
- 自动更新中的 Git Skill 更新。

## 实现方案

- 前端读取已有 `githubProxyConfig.url`，生成 updater 的 `proxy` 参数。
- 应用启动时自动检查更新和设置页手动检查更新都传入同一 proxy。
- 下载并安装更新时也传入同一 proxy。
- release notes 改为后端命令请求 GitHub Release API，复用后端统一 HTTP client。
- `skills_search` 后端接口增加代理参数，复用统一 HTTP client。
- 设置页文案从“GitHub 操作”调整为“应用外部联网请求”。

## 验收标准

- 代理开启时，自动更新检查和设置页更新检查使用 `proxy` 参数。
- 代理开启时，更新下载和安装使用 `proxy` 参数。
- release notes 不再使用前端 `fetch` 直连 GitHub API。
- Explore 在线搜索不再直接 `Client::new()`。
- `npm run check` 通过。
