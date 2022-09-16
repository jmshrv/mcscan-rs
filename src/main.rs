use std::error::Error;
use std::fmt::Write;
use std::io;

use async_minecraft_ping::{ConnectionConfig, PingConnection, ServerError, StatusResponse};
use chrono::Utc;
use clap::Parser;
use futures::{FutureExt, StreamExt};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Scans and logs Minecraft servers from a given Masscan output
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input file for Masscan output
    #[clap(short, long, value_parser)]
    input: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CombinedServerStatus {
    address: String,
    port: u16,
    status: StatusResponse,
}

async fn load_servers(path: &str) -> Result<Vec<ConnectionConfig>, Box<dyn Error>> {
    let file = File::open(path).await?;
    let mut lines = BufReader::new(file).lines();

    let mut servers = Vec::new();

    while let Some(line) = lines.next_line().await? {
        if line.starts_with('#') {
            continue;
        }

        let mut line_split = line.split_whitespace();

        let port = line_split
            .nth(2)
            .ok_or(format!("Failed to get port from line {}", line))?
            .parse::<u16>()?;
        let server = line_split
            .nth(0)
            .ok_or(format!("Failed to get server from line {}", line))?;

        let connection = ConnectionConfig::build(server).with_port(port);

        servers.push(connection);
    }

    Ok(servers)
}

async fn check_server(config: ConnectionConfig) -> Result<PingConnection, ServerError> {
    let status = config
        .connect()
        .then(|connection| async move { connection?.status().await })
        .await?;

    Ok(status)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let start = Utc::now();

    let servers = load_servers(&args.input).await.unwrap();

    let mut servers_done: u64 = 0;
    let mut servers_error: u64 = 0;

    let pb = ProgressBar::new(servers.len().try_into().unwrap());
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({eta_precise})")
    .unwrap()
    .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()));

    let mut stream = tokio_stream::iter(servers)
        .map(|config| check_server(config))
        .buffer_unordered(256);

    pb.set_position(0);

    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_writer(io::stdout());

    while let Some(response) = stream.next().await {
        servers_done += 1;

        if let Ok(server) = response {
            let combined_status = CombinedServerStatus {
                address: server.address().to_string(),
                port: server.port(),
                status: server.status,
            };

            csv_writer
                .serialize(combined_status)
                .expect("Failed to serialise to CSV");
        } else {
            servers_error += 1;
        }

        pb.set_position(servers_done);
    }

    csv_writer.flush().expect("Failed to flush CSV");

    let time_taken = Utc::now() - start;

    eprintln!(
        "Processed {} servers in {:02}:{:02}:{:02} ({} failed)",
        servers_done,
        time_taken.num_hours(),
        time_taken.num_minutes() % 60,
        time_taken.num_seconds() % 60,
        servers_error
    );
}
