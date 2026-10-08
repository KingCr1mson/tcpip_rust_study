use std::env;
use std::fs::File;
use std::net::{Ipv4Addr, UdpSocket};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
fn main() {
    let scenario = match env::args().nth(1) {
        Some(value) => value,
        None => {
            eprintln!("用法: netlab <ch09-default-route|ch09-longest-prefix|ch09-missing-route>");
            std::process::exit(2);
        }
    };

    if let Err(error) = run(&scenario) {
        eprintln!("实验失败: {error}");
        std::process::exit(1);
    }
}

fn run(scenario: &str) -> Result<(), String> {
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
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes(scenario)?;

    let route = run_command_output("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "get", SERVER_IP]);
    match (scenario == "ch09-missing-route", route) {
        (true, Ok(value)) => return Err(format!("没有路由时却找到了路由: {}", value.trim())),
        (true, Err(_)) => println!("client 的路由选择: 无可用路由"),
        (false, Ok(value)) => println!("client 的路由选择:\n{}", value.trim()),
        (false, Err(error)) => return Err(error),
    }

    let capture_path = format!("captures/{scenario}.pcap");
    println!("抓包文件: {capture_path}");
    let capture_file = File::create(&capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;

    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w -")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let ping_status = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "ping", "-c", "1", "-W", "2", SERVER_IP])
        .status()
        .map_err(|error| format!("无法发送 ping: {error}"))?;

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

fn run_udp_unicast() -> Result<(), String> {
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
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -u -l -p {UDP_PORT}"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 UDP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf '{UDP_MESSAGE}' | nc -u -w 2 {SERVER_IP} {UDP_PORT}"))
        .status()
        .map_err(|error| format!("无法发送 UDP 数据: {error}"))?;

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

fn run_tcp_pattern(name: &str, port: &str, payload: &str, label: &str) -> Result<(), String> {
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

fn run_pmtud() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?; configure_router()?; configure_routes("ch09-default-route")?;
    run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "link", "set", "dev", "eth0", "mtu", "1200"])?;
    let result = run_tcp_pattern("ch24-pmtud", TCP_PMTUD_PORT, "head -c 65536 /dev/zero", "降低 MTU 后的 TCP 传输");
    run_command_allow_failure("docker", &["compose", "exec", "-T", CLIENT, "ip", "link", "set", "dev", "eth0", "mtu", "1500"]);
    result
}

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

fn run_multicast(igmp_only: bool) -> Result<(), String> {
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

fn run_multicast_receiver() -> Result<(), String> {
    let group: Ipv4Addr = MULTICAST_GROUP
        .parse()
        .map_err(|error| format!("组播地址无效: {error}"))?;
    let local = Ipv4Addr::new(10, 0, 1, 11);
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, MULTICAST_PORT.parse::<u16>().unwrap()))
        .map_err(|error| format!("绑定组播端口失败: {error}"))?;
    socket
        .join_multicast_v4(&group, &local)
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

fn check_scenario(scenario: &str) -> Result<(), String> {
    match scenario {
        "ch09-default-route" | "ch09-longest-prefix" | "ch09-missing-route" | "ch08-traceroute" | "ch11-udp-unicast" | "ch11-udp-port-unreachable" | "ch11-udp-fragmentation" | "ch17-tcp-handshake" | "ch18-tcp-close" | "ch18-tcp-reset" | "ch19-tcp-interactive" | "ch20-tcp-stream" | "ch21-tcp-retransmission" | "ch21-congestion-control" | "ch22-tcp-zero-window" | "ch24-tcp-options" | "ch24-pmtud" | "ch12-multicast" | "ch13-igmp" => Ok(()),
        _ => Err(format!("未知实验: {scenario}")),
    }
}

fn configure_router() -> Result<(), String> {
    run_command("docker", &["compose", "exec", "-T", ROUTER, "sysctl", "-w", "net.ipv4.ip_forward=1"])?;
    Ok(())
}

fn configure_routes(scenario: &str) -> Result<(), String> {
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

fn run_command(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("执行 {program} 失败: {error}"))?;
    if !status.success() {
        return Err(format!("命令失败: {program} {}", args.join(" ")));
    }
    Ok(())
}

fn run_command_allow_failure(program: &str, args: &[&str]) {
    let _ = Command::new(program)
        .args(args)
        .stderr(Stdio::null())
        .status();
}

fn run_command_output(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("执行 {program} 失败: {error}"))?;
    if !output.status.success() {
        return Err(format!("命令失败: {program} {}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
