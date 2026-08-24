use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use extender_client::ExtenderClient;
use extender_common::protocol::{DEFAULT_INPUT_PORT, DEFAULT_STREAM_PORT, VideoCodec};
use extender_server::ExtenderServer;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "extender", version = "0.1.0", about = "Wayland-native remote extended monitor for Ubuntu GNOME")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CodecArg {
    H264Vaapi,
    H264Nvenc,
    H264Software,
    H265Vaapi,
    H265Nvenc,
    Vp8,
    Vp9,
    Av1,
}

impl From<CodecArg> for VideoCodec {
    fn from(arg: CodecArg) -> Self {
        match arg {
            CodecArg::H264Vaapi => VideoCodec::H264Vaapi,
            CodecArg::H264Nvenc => VideoCodec::H264Nvenc,
            CodecArg::H264Software => VideoCodec::H264Software,
            CodecArg::H265Vaapi => VideoCodec::H265Vaapi,
            CodecArg::H265Nvenc => VideoCodec::H265Nvenc,
            CodecArg::Vp8 => VideoCodec::Vp8,
            CodecArg::Vp9 => VideoCodec::Vp9,
            CodecArg::Av1 => VideoCodec::Av1,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Extender Host server (creates virtual output and streams)
    Host {
        #[arg(short, long, default_value_t = 1920)]
        width: u32,
        #[arg(short, long, default_value_t = 1080)]
        height: u32,
        #[arg(long, default_value_t = DEFAULT_STREAM_PORT)]
        stream_port: u16,
        #[arg(long, default_value_t = DEFAULT_INPUT_PORT)]
        input_port: u16,
        #[arg(short, long, value_enum, default_value_t = CodecArg::H264Software)]
        codec: CodecArg,
        #[arg(short, long, default_value_t = 8000)]
        bitrate_kbps: u32,
    },
    /// Connect to an Extender Host from the client laptop
    Client {
        #[arg(short, long)]
        server: SocketAddr,
        #[arg(short, long, default_value_t = 1920)]
        width: u32,
        #[arg(short, long, default_value_t = 1080)]
        height: u32,
        #[arg(short, long, default_value_t = 60)]
        refresh_rate: u32,
        #[arg(short, long, value_enum, default_value_t = CodecArg::H264Software)]
        codec: CodecArg,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Host {
            width,
            height,
            stream_port,
            input_port,
            codec,
            bitrate_kbps,
        } => {
            info!("Starting Extender Host ({width}x{height}) on port {input_port}...");
            let mut server = ExtenderServer::new(
                width,
                height,
                stream_port,
                input_port,
                codec.into(),
                bitrate_kbps,
            )
            .await?;
            server.run().await?;
        }
        Commands::Client {
            server,
            width,
            height,
            refresh_rate,
            codec,
        } => {
            info!("Starting Extender Client connecting to {server}...");
            let client = ExtenderClient::new(
                server,
                width,
                height,
                refresh_rate,
                vec![codec.into(), VideoCodec::H264Software],
            );
            client.connect_and_run().await?;
        }
    }

    Ok(())
}
