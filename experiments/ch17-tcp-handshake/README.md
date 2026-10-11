# 第 17 章：TCP 传输控制协议

## 学习目标

观察 TCP 首部、三次握手和可靠字节流中的序列号、确认号、窗口与标志位。

## 运行

```bash
make experiment EXP=ch17-tcp-handshake
```

抓包文件：`captures/ch17-tcp-handshake.pcap`。

```bash
tcpdump -r captures/ch17-tcp-handshake.pcap -nn -e -v
```

## 实验流程

server 监听 TCP/9003；client 通过 `nc` 连接 server 并发送一行文本。Rust 控制程序先启动抓包，再启动 server，最后启动 client，确保握手不会早于抓包开始。

## 三次握手

1. client -> server：SYN，携带初始序列号和 TCP 选项。
2. server -> client：SYN/ACK，确认 client 的 SYN，并发送自己的初始序列号。
3. client -> server：ACK，确认 server 的 SYN；连接进入 ESTABLISHED。

SYN 各消耗一个序列号，因此第三个 ACK 的确认号通常是 server 初始序列号加 1。

## 首部字段

- `sport/dport`：端口，标识应用端点。
- `seq`：本方向字节流位置。
- `ack`：期望收到的下一个字节。
- `win`：接收方当前可接收窗口，经过 Window Scale 后才是实际窗口。
- `Flags [S]`、`[S.]`、`[.]`：SYN、SYN/ACK、普通 ACK。
- `options`：MSS、SACK permitted、Window Scale、Timestamp 等协商能力。

## 可靠性边界

TCP 保证的是有序字节流，不保留应用层的“消息边界”。本实验发送一行文本只是便于验证，不应把一次 `printf` 等同于网络上的一个 TCP 包；内核可以合并或拆分写入内容。

## 代码与 Docker 操作

`run_tcp_handshake()` 先让 router 执行 `tcpdump -i eth0 -U -w - 'tcp port 9003'`，再启动 server 的 `nc -l -p 9003` 和 client 的 `printf 'tcp handshake chapter 17\n' | nc -w 2 10.0.2.10 9003`。Rust 用 `spawn()` 让抓包和 server 后台运行，用 `wait_with_output()` 读取 server 收到的文本。

结束后程序执行 `tcpdump -r ... -nn`，用 `contains("Flags [S]")`、`contains("Flags [S.]")` 和 `contains("Flags [.]")` 检查 SYN、SYN/ACK、ACK。真正的 TCP 字段解析由 tcpdump 完成，Rust 这里只做教学用字符串验证。
