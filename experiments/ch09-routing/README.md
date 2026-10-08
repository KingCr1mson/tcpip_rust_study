# 第 9 章：IP 选路

## 学习目标

- 观察 client 如何选择默认路由
- 观察最长前缀匹配路由
- 观察没有可用路由时的失败
- 在 router 的 client 侧接口观察转发前的报文
- 对比经过路由器前后的 TTL 和 Ethernet 地址

## 运行

在仓库根目录执行：

```bash
make experiment EXP=ch09-default-route
make experiment EXP=ch09-longest-prefix
make experiment EXP=ch09-missing-route
```

程序会创建三个容器、配置路由、抓包并发送一次 ICMP 请求。抓包保存在 `captures/`。

查看抓包：

```bash
tcpdump -r captures/ch09-default-route.pcap -nn -e
```

也可以进入容器手工观察：

```bash
docker compose exec client ip route
docker compose exec router ip route
docker compose exec server ip route
```

## 预期现象

`ch09-default-route` 中，client 没有到 `10.0.2.0/24` 的专用路由，因此使用默认路由 `via 10.0.1.2`。报文经过 router 后，TTL 减少 1，每个二层网络上的 Ethernet 地址不同。

`ch09-longest-prefix` 中，client 同时有默认路由和 `10.0.2.0/24` 路由。对 `10.0.2.10` 的查询应选择更长的 `/24` 路由。

## 第 11 章实验

### UDP 单播

```bash
make experiment EXP=ch11-udp-unicast
```

这个实验使用 client 向 server 的 UDP/9000 发送一段文本。router 的 client 侧接口只抓 UDP 报文，结果保存为 `captures/ch11-udp-unicast.pcap`。

预期观察：

- UDP 没有握手过程
- 报文包含源端口、目的端口和长度
- IPv4 目的地址保持为 `10.0.2.10`
- 报文经过 router 后 TTL 减少
- 每个二层网络的 Ethernet 地址不同

查看报文：

### UDP 分片

```bash
make experiment EXP=ch11-udp-fragmentation
```

实验将 client 的 MTU 改为 1200，并发送 3000 字节 UDP 数据。抓包保存在 `captures/ch11-udp-fragmentation.pcap`。

预期观察：

- 一个 UDP 数据报被拆成多个 IPv4 分片
- 首片包含 UDP 首部，后续分片由 IPv4 identification 和 fragment offset 关联
- 除最后一个分片外，More Fragments 标志置位
- server 收到并重组出完整的 3000 字节 UDP payload

查看分片字段：

```bash
tcpdump -r captures/ch11-udp-fragmentation.pcap -nn -e -v
```

```bash
make experiment EXP=ch11-udp-port-unreachable
```

这个实验不在 server 上监听 UDP/9001。client 发送 UDP 数据后，server 内核应返回 ICMP Destination Unreachable / Port Unreachable。router 的 client 侧接口抓取 UDP 请求和返回的 ICMP 报文。

预期观察：

- client 先发送 UDP 数据报
- server 没有 UDP/9001 监听者
- server 返回 ICMP 端口不可达
- ICMP 报文经过 router 返回 client

查看抓包：


## 第 12、13 章实验

### IPv4 多播和 IGMP

```bash
make experiment EXP=ch12-multicast
make experiment EXP=ch13-igmp
```

`multicast_receiver` 与 client 位于同一二层网络。receiver 使用 Rust `UdpSocket::join_multicast_v4` 加入 `239.1.1.1`，client 发送 UDP/9008，router 的 client 侧接口抓取 IGMP 和 UDP。两个实验分别保存为 `ch12-multicast.pcap` 与 `ch13-igmp.pcap`。

预期观察：

- receiver 加组时产生 IGMPv3 Membership Report
- IPv4 组播地址 `239.1.1.1` 映射到 Ethernet 地址 `01:00:5e:01:01:01`
- client 发送 TTL 为 1 的 UDP 组播数据，加入组的 receiver 收到数据
- Docker bridge 实验说明同一二层组播，不代表跨路由组播转发

查看报文：

```bash
tcpdump -r captures/ch12-multicast.pcap -nn -e -v
```

## 第 8、19、22、24 章实验

```bash
make experiment EXP=ch08-traceroute
make experiment EXP=ch19-tcp-interactive
make experiment EXP=ch22-tcp-zero-window
make experiment EXP=ch24-tcp-options
make experiment EXP=ch24-pmtud
make experiment EXP=ch21-congestion-control
```

这些实验分别抓取 TTL 超时、TCP 小数据交互、接收窗口变化、SYN 选项、降低 MTU 后的 TCP 传输，以及拥塞控制大流。对应 pcap 位于 `captures/`；使用 `tcpdump -r ... -nn -e -v` 查看 TTL、MSS、窗口扩大、时间戳、SACK 和重传字段。零窗口是否出现取决于宿主机内核和 socket 缓冲区，拥塞控制实验使用当前内核默认算法，需配合 `ss -ti` 或修改容器内 `net.ipv4.tcp_congestion_control` 做 Reno/CUBIC 对比。


### TCP 三次握手

```bash
make experiment EXP=ch17-tcp-handshake
```

server 监听 TCP/9003，client 建立连接并发送一行文本。router 抓包保存为 `captures/ch17-tcp-handshake.pcap`。

预期观察：

- client 发送 SYN
- server 返回 SYN/ACK
- client 返回 ACK
- TCP 连接建立后传输应用数据
- TCP 报文包含序列号、确认号和窗口字段


### TCP 连接终止

```bash
make experiment EXP=ch18-tcp-close
```

client 发送数据后关闭 TCP 连接，server 读到 EOF 后退出。router 抓包保存为 `captures/ch18-tcp-close.pcap`。

预期观察：

- 主动关闭方发送 FIN
- 对端返回 ACK
- 对端随后发送 FIN
- 主动关闭方返回最后一个 ACK
- 这对应 TCP 的四次挥手过程


### TCP RST

```bash
make experiment EXP=ch18-tcp-reset
```

server 不监听 TCP/9005。client 尝试建立连接后，server 内核应直接返回 RST，表示该端口没有监听者。

预期观察：

- client 发送 SYN
- server 返回 RST/ACK
- 不会完成三次握手
- 不会出现应用数据或 FIN 挥手

查看抓包：


## 第 20 章实验

### TCP 成块数据流

```bash
make experiment EXP=ch20-tcp-stream
```

client 通过 TCP 向 server 发送 1 MiB 数据，server 统计收到的字节数。router 抓包保存为 `captures/ch20-tcp-stream.pcap`。

预期观察：

- TCP 通过多个报文传输连续的序列空间
- ACK 号随着数据接收推进
- TCP 首部包含 advertised window
- server 最终收到完整的 1048576 字节

查看抓包：

```bash
tcpdump -r captures/ch20-tcp-stream.pcap -nn -e -v
```


## 第 21 章实验

### TCP 超时与重传

```bash
make experiment EXP=ch21-tcp-retransmission
```

实验在 TCP 连接建立后，使用 router 上的 `tc netem loss 30%` 注入丢包，再由 client 发送 65536 字节数据。server 统计接收长度，router 抓包保存为 `captures/ch21-tcp-retransmission.pcap`。

预期观察：

- 某个 TCP 数据段或 ACK 被丢弃
- 后续报文中再次出现相同的序列号范围
- 重传间隔由 TCP 定时器和 RTT 估计影响
- server 最终仍收到完整数据

查看抓包：

```bash
tcpdump -r captures/ch21-tcp-retransmission.pcap -nn -e -v
```
