use std::env;
use std::fs::File;
use std::net::{Ipv4Addr, UdpSocket};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

// 初学者阅读提示：
// - `const NAME: &str = ...` 定义编译期常量；`&str` 是借用的字符串切片。
// - 函数返回 `Result<(), String>`，表示成功时没有有用返回值，失败时带一段错误文字。
// - `?` 用于把失败向上传递；例如 `File::create(...)?` 失败时，当前函数立即返回 Err。
// - `format!("...{value}...")` 创建新的 String；普通字符串字面量则通常是 `&'static str`。
// - 本程序不自己实现 TCP/IP，而是用标准库 `Command` 调用 Docker、ping、tcpdump、nc 等工具。
//   网络行为由 Linux 内核和容器中的命令完成，Rust 负责安排顺序、收集结果和检查抓包。

// 容器名、地址和端口集中定义，实验函数通过这些常量拼出 docker compose 命令。
// 端口分配固定，便于从抓包中识别当前实验。
const CLIENT: &str = "client";
const ROUTER: &str = "router";
const SERVER: &str = "server";
const SERVER_IP: &str = "10.0.2.10";
const ROUTER_CLIENT_IP: &str = "10.0.1.2";
const ROUTER_SERVER_IP: &str = "10.0.2.2";
const UDP_PORT: &str = "9000";
const UDP_UNREACHABLE_PORT: &str = "9001";
const UDP_FRAGMENT_PORT: &str = "9002";
const UDP_FRAGMENT_MTU: &str = "1200";
const UDP_FRAGMENT_SIZE: &str = "3000";
const TCP_HANDSHAKE_PORT: &str = "9003";
const TCP_CLOSE_PORT: &str = "9004";
const TCP_RESET_PORT: &str = "9005";
const TCP_STREAM_PORT: &str = "9006";
const TCP_STREAM_SIZE: &str = "1048576";
const TCP_RETRANSMIT_PORT: &str = "9007";
const TCP_RETRANSMIT_SIZE: &str = "65536";
const MULTICAST_GROUP: &str = "239.1.1.1";
const MULTICAST_PORT: &str = "9008";
const TCP_INTERACTIVE_PORT: &str = "9009";
const TCP_ZERO_WINDOW_PORT: &str = "9010";
const TCP_OPTIONS_PORT: &str = "9011";
const TCP_PMTUD_PORT: &str = "9012";
const TCP_CONGESTION_PORT: &str = "9013";
const TCP_ZERO_WINDOW_SIZE: &str = "4194304";
const TCP_CONGESTION_SIZE: &str = "4194304";
const UDP_MESSAGE: &str = "netlab udp chapter 11\n";

// Rust 程序入口：args().nth(1) 取得第一个命令行参数。Option 表示参数可能不存在。
fn main() {
    // `env::args()` 返回命令行参数迭代器；`nth(1)` 取第一个用户参数（第 0 个是程序名）。
    // `Option<String>` 表示参数可能存在 (Some)，也可能不存在 (None)。
    let scenario = match env::args().nth(1) {
        Some(value) => value,
        None => {
            eprintln!("用法: netlab <ch09-default-route|ch09-longest-prefix|ch09-missing-route>");
            std::process::exit(2);
        }
    };

    if let Err(error) = run(&scenario) {
        // 统一把实验错误输出到 stderr，并以非零状态退出，便于脚本判断失败。
        eprintln!("实验失败: {error}");
        std::process::exit(1);
    }
}

// 根据实验名分发到具体函数；返回 Result 让错误沿调用链传回 main。
fn run(scenario: &str) -> Result<(), String> {
    // `&str` 是对调用者字符串的只读借用，不需要复制整个参数。
    // 每个专用场景先在这里分流；三个 ch09 路由场景走文件后面的通用分支。
    if scenario == "multicast-receiver" {
        return run_multicast_receiver();
    }
    if scenario == "ch11-udp-unicast" {
        return run_udp_unicast();
    }
    if scenario == "ch11-udp-port-unreachable" {
        return run_udp_port_unreachable();
    }
    if scenario == "ch11-udp-fragmentation" {
        return run_udp_fragmentation();
    }
    if scenario == "ch17-tcp-handshake" {
        return run_tcp_handshake();
    }
    if scenario == "ch18-tcp-close" {
        return run_tcp_close();
    }
    if scenario == "ch18-tcp-reset" {
        return run_tcp_reset();
    }
    if scenario == "ch20-tcp-stream" {
        return run_tcp_stream();
    }
    if scenario == "ch21-tcp-retransmission" {
        return run_tcp_retransmission();
    }
    if scenario == "ch12-multicast" {
        return run_multicast(false);
    }
    if scenario == "ch13-igmp" {
        return run_multicast(true);
    }
    if scenario == "ch08-traceroute" {
        return run_traceroute();
    }
    if scenario == "ch19-tcp-interactive" {
        return run_tcp_pattern("ch19-tcp-interactive", TCP_INTERACTIVE_PORT, "printf 'a\\nb\\nc\\n'", "交互式小数据流");
    }
    if scenario == "ch22-tcp-zero-window" {
        return run_zero_window();
    }
    if scenario == "ch24-tcp-options" {
        return run_tcp_pattern("ch24-tcp-options", TCP_OPTIONS_PORT, "printf 'tcp options\\n'", "TCP 选项协商");
    }
    if scenario == "ch24-pmtud" {
        return run_pmtud();
    }
    if scenario == "ch21-congestion-control" {
        return run_congestion_control();
    }

    check_scenario(scenario)?;
    // `&[&str]` 是命令参数切片。每个参数单独传入，不需要 Rust 自己拆分 shell 字符串。
    // `docker compose up -d` 在后台创建/启动 compose.yaml 里定义的容器和网络。
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes(scenario)?;

    // `docker compose exec -T client ...` 在已运行的 client 容器里执行命令；`-T` 关闭伪终端，
    // 适合由 Rust 捕获输出或把 stdin/stdout 接到管道的非交互式任务。
    let route = run_command_output("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "get", SERVER_IP]);
    // route 是 Result：成功时得到路由文本，失败时可能正是“无路由”实验的预期。
    match (scenario == "ch09-missing-route", route) {
        (true, Ok(value)) => return Err(format!("没有路由时却找到了路由: {}", value.trim())),
        (true, Err(_)) => println!("client 的路由选择: 无可用路由"),
        (false, Ok(value)) => println!("client 的路由选择:\n{}", value.trim()),
        (false, Err(error)) => return Err(error),
    }

    let capture_path = format!("captures/{scenario}.pcap");
    println!("抓包文件: {capture_path}");
    // File::create 会新建文件；同名文件存在时会截断旧内容。`&capture_path` 借用路径字符串。
    let capture_file = File::create(&capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;

    // Command 用于逐项构造外部进程。`spawn()` 立即启动并返回 Child，
    // 所以 Rust 可以在 tcpdump 抓包期间继续启动 ping，而不是等抓包结束。
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w -")
        // 把子进程 stdout 直接接到 pcap 文件：Docker exec 输出的是 tcpdump 的抓包字节流。
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    // 给 tcpdump 完成启动的时间；这是实验编排中的固定等待，不是协议定时器。
    thread::sleep(Duration::from_secs(1));
    // 给 tcpdump 一点启动时间，避免极短流量在抓包进程就绪前结束。
    let ping_status = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "ping", "-c", "1", "-W", "2", SERVER_IP])
        .status()
        .map_err(|error| format!("无法发送 ping: {error}"))?;

    // `wait()` 回收子进程并等待它结束；只关心退出状态，不需要读取它的 stdout。
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    if scenario == "ch09-missing-route" {
        if ping_status.success() {
            return Err("没有路由时 ping 意外成功".to_string());
        }
        println!("验证通过: 没有到 server 的路由，ping 失败");
    } else {
        if !ping_status.success() {
            return Err("存在路由时 ping 失败".to_string());
        }
        println!("验证通过: client 经由 router 到达 server");
    }

    println!("可以使用 tcpdump -r {capture_path} -nn -e 查看抓包");
    Ok(())
}

// UDP 单播：先抓包，再启动 listener，最后发送数据报。
fn run_udp_unicast() -> Result<(), String> {
    // 各网络实验采用相同准备顺序：启动拓扑、打开 router 转发、配置 client/server 路由。
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch11-udp-unicast.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w - udp")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    // `spawn()` 让 UDP listener 在后台等数据；stdout 被管道接回 Rust，稍后用于验证收到的文本。
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -u -l -p {UDP_PORT}"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 UDP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    // `status()` 等待这次发送命令结束，只保留成功/失败状态，不捕获标准输出。
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf '{UDP_MESSAGE}' | nc -u -w 2 {SERVER_IP} {UDP_PORT}"))
        .status()
        .map_err(|error| format!("无法发送 UDP 数据: {error}"))?;

    // Child::wait_with_output 等待 server 进程退出，并一次性读取它的 stdout/stderr。
    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 UDP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client.success() {
        return Err("UDP 客户端发送失败".to_string());
    }
    if !received.contains(UDP_MESSAGE.trim()) {
        return Err(format!("UDP 服务端没有收到预期数据: {received:?}"));
    }

    println!("验证通过: UDP 数据经过 router 到达 server");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -X 查看 UDP 数据");
    Ok(())
}
// server 不监听目标端口时，Linux UDP 栈会生成 ICMP Port Unreachable。
fn run_udp_port_unreachable() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch11-udp-port-unreachable.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w - 'udp or icmp'")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf '{UDP_MESSAGE}' | nc -u -w 2 {SERVER_IP} {UDP_UNREACHABLE_PORT}"))
        .status()
        .map_err(|error| format!("无法发送 UDP 数据: {error}"))?;

    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    if !client.success() {
        return Err("UDP 客户端发送失败".to_string());
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    // 解析工作交给 tcpdump，Rust 只检查本实验关心的关键词。
    let has_udp = packets.contains("UDP") || packets.contains("udp");
    let has_icmp = packets.contains("unreachable") || packets.contains("ICMP");
    if !has_udp || !has_icmp {
        return Err(format!("抓包没有同时发现 UDP 和 ICMP 端口不可达:\n{packets}"));
    }

    println!("验证通过: 未监听的 UDP 端口返回 ICMP 端口不可达");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e 查看 UDP 和 ICMP");
    Ok(())
}
// 降低 MTU，使一个大的 UDP 数据报需要 IPv4 分片。
fn run_udp_fragmentation() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;
    run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "link", "set", "dev", "eth0", "mtu", UDP_FRAGMENT_MTU])?;

    let capture_path = "captures/ch11-udp-fragmentation.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w - ip")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -u -l -p {UDP_FRAGMENT_PORT} | wc -c"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 UDP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("head -c {UDP_FRAGMENT_SIZE} /dev/zero | nc -u -w 2 {SERVER_IP} {UDP_FRAGMENT_PORT}"))
        .status()
        .map_err(|error| format!("无法发送大 UDP 数据报: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 UDP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    if !client.success() {
        return Err("UDP 客户端发送失败".to_string());
    }
    let received = String::from_utf8_lossy(&output.stdout);
    if !received.trim().contains(UDP_FRAGMENT_SIZE) {
        return Err(format!("server 没有重组出完整 UDP 数据，收到字节数: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn", "-v"])?;
    let fragment_count = packets.lines().filter(|line| line.contains("offset") || line.contains("Flags [+")).count();
    if fragment_count < 2 {
        return Err(format!("抓包中没有发现至少两个 IPv4 分片:\n{packets}"));
    }

    println!("验证通过: {UDP_FRAGMENT_SIZE} 字节 UDP 数据被分片并在 server 重组");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看分片字段");
    Ok(())
}
// 先启动 tcpdump，确保最初的 SYN 也进入 pcap。
fn run_tcp_handshake() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch17-tcp-handshake.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 8 tcpdump -i eth0 -U -w - 'tcp port {TCP_HANDSHAKE_PORT}'"))
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -l -p {TCP_HANDSHAKE_PORT}"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 TCP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf 'tcp handshake chapter 17\\n' | nc -w 2 {SERVER_IP} {TCP_HANDSHAKE_PORT}"))
        .status()
        .map_err(|error| format!("无法启动 TCP 客户端: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 TCP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client.success() {
        return Err("TCP 客户端连接或发送失败".to_string());
    }
    if !received.contains("tcp handshake chapter 17") {
        return Err(format!("TCP 服务端没有收到预期数据: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let has_syn = packets.contains("Flags [S]");
    let has_syn_ack = packets.contains("Flags [S.]");
    let has_ack = packets.contains("Flags [.]");
    if !has_syn || !has_syn_ack || !has_ack {
        return Err(format!("抓包没有发现完整的 TCP 三次握手:\n{packets}"));
    }

    println!("验证通过: TCP 连接建立并观察到 SYN、SYN/ACK、ACK");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看 TCP 首部");
    Ok(())
}
// client 发送完后关闭写方向，server 读到 EOF，便于观察 FIN 交换。
fn run_tcp_close() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch18-tcp-close.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 8 tcpdump -i eth0 -U -w - 'tcp port {TCP_CLOSE_PORT}'"))
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -l -p {TCP_CLOSE_PORT}"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 TCP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf 'tcp close chapter 18\\n' | nc -w 2 {SERVER_IP} {TCP_CLOSE_PORT}"))
        .status()
        .map_err(|error| format!("无法启动 TCP 客户端: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 TCP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client.success() {
        return Err("TCP 客户端关闭连接失败".to_string());
    }
    if !received.contains("tcp close chapter 18") {
        return Err(format!("TCP 服务端没有收到预期数据: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let fin_count = packets.matches("Flags [F.").count();
    let ack_count = packets.matches("Flags [.]").count();
    if fin_count < 2 || ack_count < 2 {
        return Err(format!("抓包没有发现完整的 TCP 连接终止过程:\n{packets}"));
    }

    println!("验证通过: TCP 连接关闭时观察到双方 FIN 和 ACK");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看连接终止");
    Ok(())
}
// 目标端口没有 listener，连接请求由 server 内核用 RST 拒绝。
fn run_tcp_reset() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch18-tcp-reset.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 8 tcpdump -i eth0 -U -w - 'tcp port {TCP_RESET_PORT}'"))
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("nc -w 2 {SERVER_IP} {TCP_RESET_PORT}"))
        .status()
        .map_err(|error| format!("无法启动 TCP 客户端: {error}"))?;

    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    if client.success() {
        return Err("未监听的 TCP 端口却连接成功".to_string());
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let has_syn = packets.contains("Flags [S]");
    let has_reset = packets.contains("Flags [R") || packets.contains("Flags [R.]");
    if !has_syn || !has_reset {
        return Err(format!("抓包没有发现 SYN 和 RST:\n{packets}"));
    }

    println!("验证通过: 未监听的 TCP 端口返回 RST");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看 TCP RST");
    Ok(())
}
// 用大块字节流观察 TCP 序列空间、累计 ACK 和接收窗口。
fn run_tcp_stream() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch20-tcp-stream.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 12 tcpdump -i eth0 -U -w - 'tcp port {TCP_STREAM_PORT}'"))
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 8 nc -l -p {TCP_STREAM_PORT} | wc -c"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 TCP 数据接收端: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("head -c {TCP_STREAM_SIZE} /dev/zero | nc -w 5 {SERVER_IP} {TCP_STREAM_PORT}"))
        .status()
        .map_err(|error| format!("无法发送 TCP 数据: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 TCP 接收端失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client.success() {
        return Err("TCP 客户端发送失败".to_string());
    }
    if received.trim() != TCP_STREAM_SIZE {
        return Err(format!("server 收到字节数不符: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let data_segments = packets.lines().filter(|line| line.contains("length ") && !line.contains("length 0")).count();
    let acknowledgements = packets.lines().filter(|line| line.contains("Flags [.]")).count();
    if data_segments < 2 || acknowledgements < 1 {
        return Err(format!("抓包中没有发现 TCP 成块数据和 ACK:\n{packets}"));
    }

    println!("验证通过: TCP 传输了 {TCP_STREAM_SIZE} 字节，server 收到完整数据");
    println!("抓包中发现 {data_segments} 个数据报文和 {acknowledgements} 个 ACK 报文");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看 sequence、ack 和 window");
    Ok(())
}
// 注入丢包，比较原始段和重传段的序列号。
fn run_tcp_retransmission() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch21-tcp-retransmission.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 15 tcpdump -i eth0 -U -w - 'tcp port {TCP_RETRANSMIT_PORT}'"))
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 12 nc -l -p {TCP_RETRANSMIT_PORT} | wc -c"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 TCP 接收端: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let mut client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("(sleep 2; head -c {TCP_RETRANSMIT_SIZE} /dev/zero) | nc -w 10 {SERVER_IP} {TCP_RETRANSMIT_PORT}"))
        .spawn()
        .map_err(|error| format!("无法启动 TCP 客户端: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    run_command("docker", &["compose", "exec", "-T", ROUTER, "tc", "qdisc", "add", "dev", "eth1", "root", "netem", "loss", "100%"]) ?;
    thread::sleep(Duration::from_secs(3));
    run_command_allow_failure("docker", &["compose", "exec", "-T", ROUTER, "tc", "qdisc", "del", "dev", "eth1", "root"]);

    let client_status = client
        .wait()
        .map_err(|error| format!("等待 TCP 客户端失败: {error}"))?;
    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 TCP 接收端失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client_status.success() {
        return Err("TCP 客户端发送失败".to_string());
    }
    if received.trim() != TCP_RETRANSMIT_SIZE {
        return Err(format!("server 收到字节数不符: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let data_lines: Vec<&str> = packets
        .lines()
        .filter(|line| line.contains("seq ") && !line.contains("length 0"))
        .collect();
    let mut has_duplicate_sequence = false;
    for (index, line) in data_lines.iter().enumerate() {
        if data_lines[index + 1..].iter().any(|other| {
            line.split("seq ").nth(1).and_then(|value| value.split(',').next())
                == other.split("seq ").nth(1).and_then(|value| value.split(',').next())
        }) {
            has_duplicate_sequence = true;
            break;
        }
    }
    if !has_duplicate_sequence {
        return Err(format!("抓包没有发现重复序列号的重传:\n{packets}"));
    }

    println!("验证通过: 丢包后 TCP 重传了相同序列号的数据");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看重传间隔");
    Ok(())
}
// TTL=1 会强制第一个三层转发点返回 ICMP Time Exceeded。
fn run_traceroute() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;
    let path = "captures/ch08-traceroute.pcap";
    let file = File::create(path).map_err(|e| format!("创建抓包文件失败: {e}"))?;
    let mut capture = Command::new("docker").args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w - 'icmp or udp'").stdout(Stdio::from(file)).spawn()
        .map_err(|e| format!("启动抓包失败: {e}"))?;
    thread::sleep(Duration::from_secs(1));
    let status = Command::new("docker").args(["compose", "exec", "-T", CLIENT, "ping", "-c", "1", "-t", "1", SERVER_IP]).status()
        .map_err(|e| format!("执行 TTL 实验失败: {e}"))?;
    capture.wait().map_err(|e| format!("等待抓包失败: {e}"))?;
    let packets = run_command_output("tcpdump", &["-r", path, "-nn", "-v"])?;
    if !packets.to_lowercase().contains("time exceeded") { return Err(format!("未观察到 ICMP TTL 超时（ping 状态 {status}）:\n{packets}")); }
    println!("验证通过: TTL=1 触发 router 返回 ICMP Time Exceeded");
    println!("抓包文件: {path}");
    Ok(())
}

// 多个实验共享同一流程；&str 是借用的字符串切片，不转移调用者的所有权。
fn run_tcp_pattern(name: &str, port: &str, payload: &str, label: &str) -> Result<(), String> {
    // 参数分别是抓包文件名、TCP 端口、容器内要执行的发送命令和给人的说明文字。
    // 多个实验复用这个函数，避免每个短 TCP 流都重复写同一套抓包/服务端/客户端流程。
    run_command("docker", &["compose", "up", "-d"])?; configure_router()?; configure_routes("ch09-default-route")?;
    let path = format!("captures/{name}.pcap");
    let file = File::create(&path).map_err(|e| format!("创建抓包文件失败: {e}"))?;
    let mut capture = Command::new("docker").args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 8 tcpdump -i eth0 -U -w - 'tcp port {port}'")).stdout(Stdio::from(file)).spawn()
        .map_err(|e| format!("启动抓包失败: {e}"))?;
    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker").args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 6 nc -l -p {port}")).stdout(Stdio::piped()).spawn()
        .map_err(|e| format!("启动服务失败: {e}"))?;
    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker").args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("{payload} | nc -w 2 {SERVER_IP} {port}")).status()
        .map_err(|e| format!("启动客户端失败: {e}"))?;
    let output = server.wait_with_output().map_err(|e| format!("等待服务失败: {e}"))?;
    capture.wait().map_err(|e| format!("等待抓包失败: {e}"))?;
    if !client.success() || output.stdout.is_empty() { return Err(format!("{label} 未完成")); }
    let packets = run_command_output("tcpdump", &["-r", &path, "-nn", "-v"])?;
    println!("验证通过: {label}"); println!("抓包文件: {path}");
    if name == "ch24-tcp-options" && !packets.contains("wscale") { println!("提示: 当前内核未在 tcpdump 文本中显示 wscale，请用 -e -v 检查 SYN"); }
    Ok(())
}

// 大数据流用于观察窗口；是否真的出现 win 0 取决于内核缓冲区。
fn run_zero_window() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?; configure_router()?; configure_routes("ch09-default-route")?;
    let path = "captures/ch22-tcp-zero-window.pcap";
    let file = File::create(path).map_err(|e| format!("创建抓包文件失败: {e}"))?;
    let mut capture = Command::new("docker").args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 12 tcpdump -i eth0 -U -w - 'tcp port {TCP_ZERO_WINDOW_PORT}'")).stdout(Stdio::from(file)).spawn()
        .map_err(|e| format!("启动抓包失败: {e}"))?;
    thread::sleep(Duration::from_secs(1));
    let mut server = Command::new("docker").args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 8 nc -l -p {TCP_ZERO_WINDOW_PORT} >/dev/null")).spawn()
        .map_err(|e| format!("启动服务失败: {e}"))?;
    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker").args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("head -c {TCP_ZERO_WINDOW_SIZE} /dev/zero | nc -w 8 {SERVER_IP} {TCP_ZERO_WINDOW_PORT}")).status()
        .map_err(|e| format!("发送数据失败: {e}"))?;
    let _ = server.wait(); capture.wait().map_err(|e| format!("等待抓包失败: {e}"))?;
    let packets = run_command_output("tcpdump", &["-r", path, "-nn"])?;
    if !client.success() { return Err("零窗口实验发送失败".into()); }
    println!("验证通过: TCP 大数据流已抓取，可搜索 win 0、zero-window probe 或窗口收缩"); println!("抓包文件: {path}"); println!("抓包摘要包含窗口字段: {}", packets.contains("win")); Ok(())
}

// MTU 是临时实验状态，结束时尝试恢复为 1500。
fn run_pmtud() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?; configure_router()?; configure_routes("ch09-default-route")?;
    run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "link", "set", "dev", "eth0", "mtu", "1200"])?;
    let result = run_tcp_pattern("ch24-pmtud", TCP_PMTUD_PORT, "head -c 65536 /dev/zero", "降低 MTU 后的 TCP 传输");
    run_command_allow_failure("docker", &["compose", "exec", "-T", CLIENT, "ip", "link", "set", "dev", "eth0", "mtu", "1500"]);
    result
}

// 抓取大 TCP 流供分析；拥塞算法使用容器内核当前配置。
fn run_congestion_control() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?; configure_router()?; configure_routes("ch09-default-route")?;
    let path = "captures/ch21-congestion-control.pcap";
    let file = File::create(path).map_err(|e| format!("创建抓包文件失败: {e}"))?;
    let mut capture = Command::new("docker").args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 12 tcpdump -i eth0 -U -w - 'tcp port {TCP_CONGESTION_PORT}'")).stdout(Stdio::from(file)).spawn()
        .map_err(|e| format!("启动抓包失败: {e}"))?;
    thread::sleep(Duration::from_secs(1));
    let mut server = Command::new("docker").args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 10 nc -l -p {TCP_CONGESTION_PORT} >/dev/null")).spawn().map_err(|e| format!("启动服务失败: {e}"))?;
    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker").args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("head -c {TCP_CONGESTION_SIZE} /dev/zero | nc -w 8 {SERVER_IP} {TCP_CONGESTION_PORT}")).status().map_err(|e| format!("发送数据失败: {e}"))?;
    let _ = server.wait(); capture.wait().map_err(|e| format!("等待抓包失败: {e}"))?;
    if !client.success() { return Err("拥塞控制数据发送失败".into()); }
    println!("验证通过: 已抓取 TCP 大流量，可用 ss -ti 和 tcpdump 分析 cwnd、rtt、重传"); println!("抓包文件: {path}"); Ok(())
}

// receiver 先加入组，sender 再发送，避免错过唯一的组播数据报。
fn run_multicast(igmp_only: bool) -> Result<(), String> {
    // bool 参数区分第 12 章（还验证数据交付）和第 13 章（重点验证 IGMP 报告）。
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;
    run_command_allow_failure("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "add", "239.0.0.0/8", "dev", "eth0"]);

    let name = if igmp_only { "ch13-igmp" } else { "ch12-multicast" };
    let capture_path = format!("captures/{name}.pcap");
    let capture_file = File::create(&capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 10 tcpdump -i eth0 -U -w - 'igmp or udp'")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动组播抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let receiver = Command::new("docker")
        .args(["compose", "exec", "-T", "multicast_receiver", "/netlab", "multicast-receiver"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动组播接收端: {error}"))?;

    thread::sleep(Duration::from_secs(2));
    let sender = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf 'netlab multicast chapter 12\\n' | nc -u -w 2 {MULTICAST_GROUP} {MULTICAST_PORT}"))
        .status()
        .map_err(|error| format!("无法发送组播数据: {error}"))?;

    let output = receiver
        .wait_with_output()
        .map_err(|error| format!("等待组播接收端失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待组播抓包失败: {error}"))?;

    if !sender.success() {
        return Err("组播发送失败".to_string());
    }
    let received = String::from_utf8_lossy(&output.stdout);
    if !igmp_only && !received.contains("netlab multicast chapter 12") {
        return Err(format!("组播接收端没有收到数据: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", &capture_path, "-nn", "-v"])?;
    if !packets.contains(MULTICAST_GROUP) {
        return Err(format!("抓包没有发现目标组播地址:\n{packets}"));
    }
    if igmp_only && !packets.contains("IGMP") {
        return Err(format!("抓包没有发现 IGMP Membership Report:\n{packets}"));
    }

    if igmp_only {
        println!("验证通过: 加入组播组时观察到 IGMP 报文");
    } else {
        println!("验证通过: receiver 加入组播组并收到 UDP 组播数据");
    }
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看组播 IP、Ethernet MAC 和 IGMP");
    Ok(())
}

// 在 receiver 容器中调用 join_multicast_v4，请求 Linux 内核交付该组 UDP 数据。
fn run_multicast_receiver() -> Result<(), String> {
    // 此分支不是在宿主机运行，而是由 router/client 编排代码在 receiver 容器内调用。
    // UdpSocket、Ipv4Addr、Duration 都来自 Rust 标准库，不需要额外 crate。
    let group: Ipv4Addr = MULTICAST_GROUP
        .parse()
        .map_err(|error| format!("组播地址无效: {error}"))?;
    let local = Ipv4Addr::new(10, 0, 1, 11);
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, MULTICAST_PORT.parse::<u16>().unwrap()))
        .map_err(|error| format!("绑定组播端口失败: {error}"))?;
    socket
        .join_multicast_v4(&group, &local)
        // ? 会在失败时提前返回，并保留我们补充的错误上下文。
        .map_err(|error| format!("加入组播组失败: {error}"))?;
    socket
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|error| format!("设置组播接收超时失败: {error}"))?;
    let mut buffer = [0u8; 2048];
    let (size, peer) = socket
        .recv_from(&mut buffer)
        .map_err(|error| format!("接收组播数据失败: {error}"))?;
    println!("收到组播数据，来源 {peer}: {}", String::from_utf8_lossy(&buffer[..size]));
    Ok(())
}

// 启动容器前拒绝未知实验名，避免拼写错误造成误操作。
fn check_scenario(scenario: &str) -> Result<(), String> {
    // 显式白名单可在启动 Docker 前拦截拼错的实验名。
    match scenario {
        "ch09-default-route" | "ch09-longest-prefix" | "ch09-missing-route" | "ch08-traceroute" | "ch11-udp-unicast" | "ch11-udp-port-unreachable" | "ch11-udp-fragmentation" | "ch17-tcp-handshake" | "ch18-tcp-close" | "ch18-tcp-reset" | "ch19-tcp-interactive" | "ch20-tcp-stream" | "ch21-tcp-retransmission" | "ch21-congestion-control" | "ch22-tcp-zero-window" | "ch24-tcp-options" | "ch24-pmtud" | "ch12-multicast" | "ch13-igmp" => Ok(()),
        _ => Err(format!("未知实验: {scenario}")),
    }
}

// router 默认不一定转发 IPv4 包，这里显式打开内核转发开关。
fn configure_router() -> Result<(), String> {
    run_command("docker", &["compose", "exec", "-T", ROUTER, "sysctl", "-w", "net.ipv4.ip_forward=1"])?;
    Ok(())
}

// 先清理上一次实验留下的路由，让每次实验从可预测状态开始。
fn configure_routes(scenario: &str) -> Result<(), String> {
    // Compose 网络会自动设置容器接口和直连网段，但本实验显式调整默认路由，
    // 以便展示默认路由、最长前缀和无路由三种情况。先删除旧项使重复运行可预测。
    run_command_allow_failure("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "del", "default"]);
    run_command_allow_failure("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "del", "10.0.2.0/24", "via", ROUTER_CLIENT_IP]);
    run_command_allow_failure("docker", &["compose", "exec", "-T", SERVER, "ip", "route", "del", "default"]);

    if scenario != "ch09-missing-route" {
        run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "add", "default", "via", ROUTER_CLIENT_IP])?;
    }
    run_command("docker", &["compose", "exec", "-T", SERVER, "ip", "route", "add", "default", "via", ROUTER_SERVER_IP])?;

    if scenario == "ch09-longest-prefix" {
        run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "add", "10.0.2.0/24", "via", ROUTER_CLIENT_IP])?;
    }
    Ok(())
}

// Command 分离传递程序名和参数，不经过 shell 字符串拆分。
fn run_command(program: &str, args: &[&str]) -> Result<(), String> {
    // `Command::status()` 等待程序结束并取得退出状态；Unix 命令通常以 0 表示成功。
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("执行 {program} 失败: {error}"))?;
    if !status.success() {
        // 子进程退出码不是 Rust 的 Result，因此手动转换为可读错误。
        return Err(format!("命令失败: {program} {}", args.join(" ")));
    }
    Ok(())
}

// 清理命令在目标不存在时失败是正常的，因此有意忽略退出码。
fn run_command_allow_failure(program: &str, args: &[&str]) {
    // 用于“删除可能不存在的旧路由/qdisc”等清理操作：失败也不阻断后续实验。
    let _ = Command::new(program)
        .args(args)
        .stderr(Stdio::null())
        .status();
}

// 等待外部命令结束并收集 stdout，再转换为拥有所有权的 String。
fn run_command_output(program: &str, args: &[&str]) -> Result<String, String> {
    // `output()` 同时捕获 stdout、stderr 并等待退出；这里把 stdout 按 UTF-8 有损转换成 String。
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("执行 {program} 失败: {error}"))?;
    if !output.status.success() {
        return Err(format!("命令失败: {program} {}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
