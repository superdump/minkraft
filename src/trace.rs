use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

pub fn setup_global_subscriber() -> impl Drop {
    let fmt_layer = fmt::Layer::default();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("error"))
        .unwrap();

    let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();

    let subscriber = Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .with(chrome_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    _guard
}
