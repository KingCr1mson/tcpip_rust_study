# 第 18 章：TCP RST 补充实验

此实验与连接终止实验属于同一章，但单独观察异常终止。

```bash
make experiment EXP=ch18-tcp-reset
tcpdump -r captures/ch18-tcp-reset.pcap -nn -e -v
```

client 向 server 的 TCP/9005 发起 SYN，而 server 没有监听该端口。Linux 内核返回带 RST/ACK 的 TCP 报文，连接没有进入 ESTABLISHED，也不会出现应用数据和正常 FIN 挥手。

解读时关注：

- SYN 的目的端口是 9005。
- 返回方向的 TCP flags 包含 `R`，通常还包含 `.`。
- RST 不表示“重传失败”，而是明确拒绝或终止连接。
- 与正常关闭的 FIN 不同，RST 不提供有序的半关闭过程。

该实验适合理解 TCP 状态机中“收到无法接受的连接请求”这一分支。

## 代码与 Docker 操作

实现位于 `run_tcp_reset()`。Rust 启动 router 的 `tcpdump -i eth0 -U -w - 'tcp port 9005'`，但故意不在 server 启动 `nc -l`。client 执行 `nc -w 2 10.0.2.10 9005`，`status()` 返回失败是预期现象；代码只在 client 意外成功时返回错误。

抓包结束后，`run_command_output("tcpdump", &["-r", path, "-nn"])` 把 pcap 转成文本，检查 `Flags [S]` 和 `Flags [R`。Rust 没有直接解析 pcap 二进制，而是调用 tcpdump 输出摘要，再用字符串方法做验证。
