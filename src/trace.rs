use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};
use tracing_tracy::TracyLayer;

pub fn setup_global_subscriber() {
    let fmt_layer = fmt::Layer::default();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,wgpu=warn"))
        .unwrap();
    let tracy_layer = TracyLayer::new();

    let subscriber = Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .with(tracy_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
}
