# TCP/IP 协议实验室

本仓库使用 Docker 创建可重复的 Linux 网络拓扑，使用 Rust 控制实验流程并保存抓包。详细说明按章节拆分：

- [第 8 章：Traceroute 与 TTL](experiments/ch08-traceroute/README.md)
- [第 9 章：IP 选路](experiments/ch09-routing/README.md)
- [第 11 章：UDP](experiments/ch11-udp/README.md)
- [第 12 章：广播和 IPv4 多播](experiments/ch12-multicast/README.md)
- [第 13 章：IGMP](experiments/ch13-igmp/README.md)
- [第 17 章：TCP](experiments/ch17-tcp-handshake/README.md)
- [第 18 章：连接建立、终止和 RST](experiments/ch18-tcp-close/README.md)
- [第 19 章：交互数据流](experiments/ch19-tcp-interactive/README.md)
- [第 20 章：成块数据流](experiments/ch20-tcp-stream/README.md)
- [第 21 章：超时、重传和拥塞控制](experiments/ch21-tcp-retransmission/README.md)
- [第 22 章：坚持定时器与零窗口](experiments/ch22-tcp-zero-window/README.md)
- [第 24 章：TCP 选项、MTU 与 PMTUD](experiments/ch24-tcp-options/README.md)

## 环境

- Linux 主机
- Docker Engine
- Docker Compose v2
- Rust/Cargo（可选；没有 Cargo 时 Makefile 使用 Docker 构建 Rust 工具）
- router 容器需要 `privileged` 权限，以便开启 IPv4 转发和注入网络条件

## 快速开始

```bash
make check
make experiment EXP=ch09-default-route
```

所有实验的命令格式都是：

```bash
make experiment EXP=<实验名称>
```

抓包保存在 `captures/`，例如：

```bash
tcpdump -r captures/ch17-tcp-handshake.pcap -nn -e -v
```

清理容器和抓包：

```bash
make clean
```

## 拓扑

```text
client 10.0.1.10 -- router -- server 10.0.2.10
                       |
                  10.0.1.2 / 10.0.2.2
```

第 12、13 章额外启动 `multicast_receiver (10.0.1.11)`，它和 client 位于同一个二层网络。

## Rust 代码

控制程序位于 [`src/main.rs`](src/main.rs)。代码只使用标准库，重点练习：

- `Result<T, E>`、`Option<T>` 和 `?` 错误传播
- `Command` 启动 Docker、`Stdio` 重定向抓包流
- `File` 保存 pcap
- `UdpSocket` 加入多播组
- `Child::wait` 等待外部进程结束

本项目用于观察协议行为和抓包，不用于测量真实高速网卡性能。
