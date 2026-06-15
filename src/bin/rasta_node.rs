use rasta_stack::adapters::socket_transport::UdpSocketTransport;
use rasta_stack::adapters::standard_clock::StdClock;
use rasta_stack::adapters::standard_timer::StdTimer;
use rasta_stack::adapters::test::MockTransport;
use rasta_stack::application::service_interface::{ConnectionStatus, RastaService};
use rasta_stack::core::connection::RastaConfig;
use rasta_stack::core::redundancy::RedundancyConfig;
use rasta_stack::core::safety_code::SafetyCodeConfig;
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <A|B> <remote_ip>", args[0]);
        return;
    }

    let mode = &args[1];
    let remote_ip = &args[2];

    let (local_addr, remote_addr, sender_id, remote_id) = if mode == "A" {
        (
            "0.0.0.0:5000",
            format!("{}:5001", remote_ip),
            0x1234,
            0x5678,
        )
    } else if mode == "B" {
        (
            "0.0.0.0:5001",
            format!("{}:5000", remote_ip),
            0x5678,
            0x1234,
        )
    } else {
        println!("Invalid mode. Use A or B.");
        return;
    };

    println!("Starting node {}", mode);
    println!("Local address: {}", local_addr);
    println!("Remote address: {}", remote_addr);

    let transport = UdpSocketTransport::new(local_addr, &remote_addr).expect("Failed to bind UDP");
    // Replace the second redundant transport with MockTransport
    let transport_b = MockTransport::new();

    let config = RastaConfig {
        sender_id,
        remote_id,
        safety_code: SafetyCodeConfig::default(),
        redundancy: RedundancyConfig::default(),
        t_max: 2000,
        initial_seq: 0,
        heartbeat_interval_ms: 500,
        n_send_max: 16,
    };

    let mut api = RastaService::new(transport, transport_b, StdTimer::new(), StdClock, config);

    if mode == "A" {
        println!("Sending ConnectionRequest...");
        api.open_connection()
            .expect("Failed to initiate connection");
    }

    let mut last_state = api.status();
    let mut data_sent = false;
    let start_time = std::time::Instant::now();

    loop {
        if let Err(e) = api.poll() {
            println!("Error during poll: {:?}", e);
            break;
        }

        let current_state = api.status();
        if current_state != last_state {
            println!("State transition: {:?} -> {:?}", last_state, current_state);
            last_state = current_state;
        }

        if current_state == ConnectionStatus::Up {
            if mode == "A" && !data_sent {
                println!("Sending data: 'Hello from A'");
                api.send_data(b"Hello from A").expect("Failed to send data");
                data_sent = true;
            }
        }

        if mode == "A" && data_sent && start_time.elapsed() > Duration::from_secs(5) {
            println!("Graceful disconnect...");
            api.close_connection().expect("Failed to disconnect");
            break;
        }

        if mode == "B" && start_time.elapsed() > Duration::from_secs(10) {
            println!("Node B timeout, exiting.");
            break;
        }

        if current_state == ConnectionStatus::Down && mode == "A" && data_sent {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }
}
