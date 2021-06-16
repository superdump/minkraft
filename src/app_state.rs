#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppState {
    Loading,
    Preparing,
    Running,
}
