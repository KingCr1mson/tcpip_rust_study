# 第 13 章：IGMP

## 学习目标

把“主机加入多播组”对应到实际的 IGMP 成员报告，理解 IGMP 管理的是接收意愿，而不是多播数据本身。

## 运行

```bash
make experiment EXP=ch13-igmp
```

抓包文件：`captures/ch13-igmp.pcap`。

```bash
tcpdump -r captures/ch13-igmp.pcap -nn -e -v
```

## 实验流程

1. 启动 `multicast_receiver`。
2. Rust 代码绑定 UDP/9008。
3. 调用 `join_multicast_v4(239.1.1.1, 10.0.1.11)`。
4. Linux 内核向本地网络发送 IGMP Membership Report。
5. client 发送一个组播 UDP 数据报。
6. router 的 `eth0` 同时抓取 IGMP 和 UDP。

## 报文解读

IGMP 位于 IPv4 之上，通常看到：

- IPv4 协议号 2，表示 IGMP。
- 目标可能是组地址或 IGMP 使用的本地控制地址。
- Membership Report 中包含 `239.1.1.1`，表示主机希望接收该组。

随后出现的 UDP 报文协议号是 17。IGMP 报告和 UDP 数据不是同一种报文：前者管理组成员，后者承载应用数据。

## 与路由器的关系

在真实网络中，组播路由器会根据 IGMP 报告维护下游接口的组成员状态，并决定是否转发组播流。本实验没有实现组播路由协议，只观察主机发出的成员报告和同一二层网络的数据交付。

## 注意事项

Linux 版本可能发送 IGMPv2 或 IGMPv3，时间点和报告次数也可能不同。程序检查抓包中是否出现 IGMP，不把具体版本写死。

## 代码与 Docker 操作

本实验调用同一个 `run_multicast(true)`，启动顺序与第 12 章相同：router 抓 `igmp or udp`，receiver 容器执行 `/netlab multicast-receiver`，client 发送 UDP/9008。布尔参数 `true` 使程序使用 `captures/ch13-igmp.pcap`，并在 tcpdump 摘要中额外检查 `IGMP`。

第 13 章重点验证 `UdpSocket::join_multicast_v4()` 触发的内核成员报告；它不把 receiver 是否收到应用数据作为主要断言。对照 `run_multicast(false)` 可以看到同一个函数如何通过参数复用两章实验的公共流程。
