# 第 12 章：广播和 IPv4 多播

## 学习目标

观察 IPv4 多播地址、Ethernet 多播 MAC 映射，以及加入多播组后 UDP 数据如何交付给 socket。

## 拓扑

```text
client 10.0.1.10 -------- net_client -------- multicast_receiver 10.0.1.11
                              |
                           router eth0
```

client 和 receiver 在同一个 Docker 二层网络。router 只负责抓取该网络上的帧，本实验不验证跨路由组播转发。

## 运行

```bash
make experiment EXP=ch12-multicast
```

程序让 receiver 使用 Rust `UdpSocket::join_multicast_v4` 加入 `239.1.1.1`，client 向 UDP/9008 发送文本。抓包位于 `captures/ch12-multicast.pcap`。

```bash
tcpdump -r captures/ch12-multicast.pcap -nn -e -v
```

## 原理

IPv4 多播地址属于 `224.0.0.0/4`。本实验的 `239.1.1.1` 映射到 Ethernet 多播地址：

```text
01:00:5e:01:01:01
```

映射规则只使用 IPv4 组播地址的低 23 位，因此不同的 IPv4 多播地址可能映射到同一个 Ethernet MAC，网卡或内核还需要进一步过滤。

receiver 加入组时，内核建立组播成员关系，并向本地网络发送 IGMP Membership Report。发送 UDP 数据时，目的 IP 是 `239.1.1.1`，目的 Ethernet MAC 是上述组播 MAC，UDP 仍然是普通 UDP 数据报。

## 帧解读

- Ethernet 目的地址不是某个 receiver 的单播 MAC，而是组播 MAC。
- IPv4 目的地址是组播组，不是 receiver 的 `10.0.1.11`。
- UDP 源/目的端口仍按普通 UDP 解读。
- receiver 是否把数据交给应用，取决于 socket 是否加入该组。

## 与广播对比

广播通常发送给同一广播域内的所有主机；多播通过组成员关系让主机表达“我想接收这个组”。本实验只覆盖多播，不模拟 IPv4 广播。

## 限制

Docker bridge 中的组播行为受 bridge、网卡过滤和内核配置影响；本实验说明的是同一二层网络的组播接收，不等于完整的 PIM/IGMP snooping 路由网络。
