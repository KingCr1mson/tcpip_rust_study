# 第 11 章：UDP

## 学习目标

观察 UDP 无连接数据报、端口不可达和 IPv4 分片，理解 UDP 首部与 IP 分片的边界。

## 实验入口

```bash
make experiment EXP=ch11-udp-unicast
make experiment EXP=ch11-udp-port-unreachable
make experiment EXP=ch11-udp-fragmentation
```

## UDP 单播

client 使用 `nc -u` 向 `10.0.2.10:9000` 发送文本，server 使用 UDP listener 接收。UDP 没有 SYN、ACK 或连接建立阶段；应用直接交给内核一个独立数据报。

重点观察：

- IPv4 `protocol 17` 表示 UDP。
- UDP 首部包含源端口、目的端口、长度和校验和。
- 目标 IP 始终是 server；经过 router 时只有 TTL 和二层 MAC 改变。
- 一个 UDP 数据报对应一个发送操作，但底层还可能发生分片。

## UDP 端口不可达

server 不监听 UDP/9001。server 收到数据报后，由内核生成 ICMP Destination Unreachable，代码为 Port Unreachable。ICMP 负载会携带原始 IP 首部和 UDP 首部前若干字节，使发送端知道哪个 UDP 流失败。

```bash
tcpdump -r captures/ch11-udp-port-unreachable.pcap -nn -e -v
```

先找 client 到 server 的 UDP，再找反向的 ICMP。ICMP 不是 UDP 的“握手”，而是异步错误通知。

## IPv4 分片

程序把 client 的 MTU 降到 1200，并发送 3000 字节 UDP 数据报：

```bash
make experiment EXP=ch11-udp-fragmentation
tcpdump -r captures/ch11-udp-fragmentation.pcap -nn -e -v
```

IP 层会把一个大 datagram 拆成多个 fragment。所有分片共享 Identification：

- 首片包含 UDP 首部。
- Fragment Offset 表示该片在原 IP payload 中的位置，单位是 8 字节。
- 除最后一片外，More Fragments 标志为 1。
- server 按 Identification、源/目的地址和协议号重组后，才交给 UDP。

注意：分片发生在 IP 层，UDP 首部不会出现在每一个分片里；抓包工具显示的“UDP 长度”只对首片有意义。

## 协议对应

```text
Ethernet -> IPv4 -> UDP -> 应用文本
                       |
                       +-- 无监听端口时触发 ICMP 错误
```

## 代码与 Docker 操作

三个场景分别对应 `run_udp_unicast()`、`run_udp_port_unreachable()` 和 `run_udp_fragmentation()`。它们都先启动 compose、打开 router 转发并配置路由。

单播场景由 Rust 启动 router 抓包、server 的 `nc -u -l -p 9000`，再让 client 执行 `printf 'netlab udp chapter 11\n' | nc -u -w 2 10.0.2.10 9000`。server 用 `spawn()` 后台监听，Rust 用 `wait_with_output()` 读取 stdout；`String::from_utf8_lossy()` 将收到的字节转成可搜索文本。

端口不可达场景不启动 server listener，只发送到 UDP/9001，并把抓包过滤器改成 `'udp or icmp'`，然后检查摘要同时包含 UDP 和 ICMP。分片场景额外执行 `client ip link set dev eth0 mtu 1200`，用 `head -c 3000 /dev/zero | nc -u ... 9002` 发送大数据报；server 通过 `wc -c` 输出计数，Rust 检查计数为 3000 并统计至少两个分片行。`head`、`nc`、`wc` 都是容器内 Linux 命令。
