# SSHTerm

Tauri 2 桌面原生 SSH/SFTP 客户端。支持 Linux + Windows。

## 截图

_（待补充）_

## 功能

- **SSH 终端** — 连接管理、多标签页、命令执行
- **SFTP 文件管理** — 双栏文件浏览器、上传下载、新建目录、重命名、删除
- **主机管理** — 添加/编辑/删除、分组、搜索、密码/密钥认证
- **暗色主题** — 护眼深色配色，Tokyo Night 风格
- **原生性能** — Tauri 2 (Rust) 原生二进制，~5MB 安装包

## 快速开始

```bash
# 安装 Tauri CLI
cargo install tauri-cli --version "^2.0"

# 安装前端依赖
npm install

# 开发模式
cargo tauri dev

# 发布构建
cargo tauri build
```

## 从 Release 下载

[Releases](https://github.com/你的用户名/sshterm/releases) 页面有预编译的安装包：

- **Linux**: `.deb` / `.AppImage`
- **Windows**: `.msi`

## 项目结构

```
sshterm/
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # Tauri 入口 + 命令注册
│   │   ├── ssh.rs          # SSH 连接 + SFTP
│   │   └── store.rs        # SQLite 持久化
│   ├── Cargo.toml
│   └── tauri.conf.json
├── frontend/               # 前端 UI
│   ├── index.html
│   ├── style.css
│   └── app.js
├── .github/workflows/      # CI/CD 自动编译
└── README.md
```

## 技术栈

| 层 | 技术 |
|---|------|
| 窗口 | Tauri 2.0 |
| 后端 | Rust (ssh2, rusqlite) |
| 前端 | HTML + CSS + JavaScript |
| 终端 | xterm.js |
| 数据库 | SQLite (host 持久化) |
| 编译 | GitHub Actions CI |

## 开发

```bash
# 启动开发服务 (热重载)
cargo tauri dev

# 仅构建前端
npm run build

# 发布构建
cargo tauri build
```

## 许可

MIT
