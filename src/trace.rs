use tracing_flame::FlameLayer;
use tracing_subscriber::{EnvFilter, registry::Registry, prelude::*, fmt};

pub fn setup_global_subscriber() -> impl Drop {
    let fmt_layer = fmt::Layer::default();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,wgpu=warn"))
        .unwrap();

    let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    let subscriber = Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .with(flame_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    _guard
}
