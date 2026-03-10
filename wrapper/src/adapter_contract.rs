// These constants are shared across local-api and connector targets, but each
// binary only uses a subset of them at compile time.
#[allow(dead_code)]
pub(crate) const CODEXW_LOCAL_API_VERSION: &str = "v1";
#[allow(dead_code)]
pub(crate) const CODEXW_BROKER_ADAPTER_VERSION: &str = "v1";

#[allow(dead_code)]
pub(crate) const HEADER_LOCAL_API_VERSION: &str = "X-Codexw-Local-Api-Version";
#[allow(dead_code)]
pub(crate) const HEADER_BROKER_ADAPTER_VERSION: &str = "X-Codexw-Broker-Adapter-Version";
