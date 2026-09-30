# TCP/IP 协议实验室

这个仓库使用 Docker 创建可重复的 Linux 网络拓扑，使用 Rust 编写实验控制和验证程序。

当前进度：第 9 章 IP 选路垂直切片。

## 环境

- Linux 主机
- Docker Engine
- Docker Compose v2
- Rust/Cargo（可选；没有 Cargo 时使用 Docker 构建 Rust 工具）
- router 容器需要 `privileged` 权限，以便开启 IPv4 转发

实验目标是观察协议行为和抓包，不用于测量真实高速网卡性能。

## 快速开始

```bash
make build
make experiment EXP=ch09-default-route
```

查看生成的抓包：

```bash
ls captures/
```

清理实验环境：

```bash
make down
```

## 第 9 章实验

```bash
make experiment EXP=ch09-default-route
make experiment EXP=ch09-longest-prefix
make experiment EXP=ch09-missing-route
```

拓扑如下：

```text
client (10.0.1.10) -- router -- server (10.0.2.10)
                       |          |
                  10.0.1.2    10.0.2.2
```

client 使用 `via 10.0.1.2` 经过 router。

## Rust 代码

Rust 代码尽量保持简单，当前只使用标准库。宿主机有 Cargo 时可以直接运行：

```bash
cargo run -- ch09-default-route
```

没有 Cargo 时，Makefile 会使用 Rust Docker 镜像构建工具；项目中的 `.cargo/config.toml` 会让 Cargo 使用 rsproxy sparse 镜像。
