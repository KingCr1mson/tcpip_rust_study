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

```bash
tcpdump -r captures/ch11-udp-unicast.pcap -nn -e -X
```
