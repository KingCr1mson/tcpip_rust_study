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


## 第 17、18 章实验

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

查看抓包：

```bash
tcpdump -r captures/ch17-tcp-handshake.pcap -nn -e -v
```
