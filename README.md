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

## 先看懂一次实验如何运行

在终端运行：

```bash
make experiment EXP=ch17-tcp-handshake
```

Makefile 会按顺序完成这些操作：

1. 如果宿主机有 Cargo，就运行 `cargo build`；否则启动 `rust:1.82-bookworm` 容器，在挂载到 `/work` 的项目目录中编译。
2. 执行 `docker compose build`，根据 `docker/lab/Dockerfile` 构建实验容器镜像。
3. 运行 `./target/debug/netlab ch17-tcp-handshake`，把实验名称交给 Rust 程序。

Rust 程序中的 `main` 读取实验名称，`run` 负责分发。实验函数使用 `Command::new("docker")` 执行 `docker compose ...`：

- `docker compose up -d` 在后台启动 `compose.yaml` 中的 client、router、server 和网络。
- `docker compose exec -T client ...` 在已经运行的 client 容器内执行命令；`-T` 表示不分配交互终端。
- `ip route`、`ping`、`nc`、`tcpdump`、`tc` 等是容器里的 Linux 工具，不是 Rust 函数。
- `Command::spawn()` 启动一个可以并行运行的子进程；例如先让 tcpdump 抓包，再启动 client 产生流量。
- `Command::status()` 等待命令结束并检查退出码；`Child::wait_with_output()` 还会收集子进程输出供 Rust 验证。

抓包命令使用 `tcpdump -w -` 把 pcap 字节写到标准输出。Rust 用 `Stdio::from(capture_file)` 将这个输出接到宿主机的 `captures/*.pcap` 文件。这里的 `-` 指 tcpdump 的标准输出，不是文件名。

## Rust 写法速查

- `let name = value;` 创建变量；默认不可变。需要后续调用 `wait()` 修改子进程状态时会写 `let mut child`。
- `&str` 是借用的字符串切片，`String` 是可拥有和修改的字符串。函数参数写 `scenario: &str` 表示只读取调用方传入的文本。
- `Result<T, E>` 表示操作成功得到 `T`，或失败得到 `E`。本程序用 `Result<(), String>` 表示实验成功或附带错误文字失败。
- `Option<T>` 表示值可能缺失。`Some(value)` 是有值，`None` 是无值；命令行参数可能不存在，所以 `env::args().nth(1)` 返回 Option。
- `?` 在结果为 Err 时立刻从当前函数返回错误；这让文件创建、命令执行等失败能逐层传回 `main`。
- `&[&str]` 是字符串切片组成的参数列表。把程序名和参数分别传给 `Command`，不需要 shell 帮忙拆分命令行。
- `format!("端口 {port}")` 按变量内容创建新的 String；`println!` 和 `eprintln!` 分别输出到标准输出和错误输出。

阅读时可以先从 `main` -> `run` -> 对应的 `run_*` 函数开始，再按函数里的容器命令去看 `compose.yaml` 和 `docker/lab/Dockerfile`。每个章节文档都列出了具体函数名和命令。
