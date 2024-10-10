mod merger;
mod sensor;
mod writer;

use merger::Merger;
use sensor::Sensor;
use tokio_util::sync::CancellationToken;
use writer::Writer;

use clap::Parser;
use tokio::{self, signal, sync::mpsc, task::JoinSet};

// should me more than enough for 100 Hz
const CHANNEL_SIZE: usize = 1000;

#[derive(Parser)]
#[command(version)]
struct Args {
    /// Output file path of the combined sensor data
    #[arg(short, long, default_value_t = String::from("sensors.csv"))]
    output_file: String,
}

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();
    let sensor_1hz = Sensor::new(1, |time| time as f64 * 100.0, token.clone()); // 1 Hz
    let sensor_100hz =
        Sensor::new(100, |time| f64::sin(time as f64), token.clone()); // 100 Hz

    let args = Args::parse();

    let (s1_tx, s1_rx) = mpsc::channel(CHANNEL_SIZE);
    let (s100_tx, s100_rx) = mpsc::channel(CHANNEL_SIZE);
    let (m_tx, m_rx) = mpsc::channel(CHANNEL_SIZE);

    // NOTE: the slower sensor is passed first
    let mut merger = Merger::new(s1_rx, s100_rx, token.clone());
    let mut writer = Writer::new(args.output_file, m_rx, token.clone());

    let mut tasks = JoinSet::new();

    println!("Press Ctrl-C to stop");

    tasks.spawn(async move { sensor_1hz.generate(s1_tx).await });
    tasks.spawn(async move { sensor_100hz.generate(s100_tx).await });
    tasks.spawn(async move { merger.process(m_tx).await });
    tasks.spawn(async move { writer.serialize().await });

    signal::ctrl_c().await.expect("failed to listen to Ctrl-C");
    token.cancel();

    tasks.join_all().await;
}
