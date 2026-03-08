#[derive(Debug, Clone)]
pub enum BootstrapProvider {
    StaticList,
    Https,
    Dns,
    BridgeBundle,
}
