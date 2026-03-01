//! DNS TXT record bootstrap discovery.
//!
//! The connector queries _wws._tcp.<domain> for TXT records containing
//! fallback bootstrap peer multiaddresses. Format:
//!   "v=1 peer=/dns4/bootstrap1.wws.dev/tcp/9000/p2p/12D3KooW..."

use libp2p::Multiaddr;

/// Error type for DNS bootstrap failures.
#[derive(Debug)]
pub struct DiscoveryError(pub String);

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DNS bootstrap error: {}", self.0)
    }
}

impl std::error::Error for DiscoveryError {}

/// Parse a single DNS TXT bootstrap record.
/// Format: "v=1 peer=<multiaddr>"
pub fn parse_bootstrap_txt_record(record: &str) -> Result<Multiaddr, DiscoveryError> {
    let parts: std::collections::HashMap<&str, &str> = record
        .split_whitespace()
        .filter_map(|kv| kv.split_once('='))
        .collect();

    let version = parts.get("v").copied().unwrap_or("");
    if version != "1" {
        return Err(DiscoveryError(format!(
            "unsupported or missing bootstrap TXT version: '{version}'"
        )));
    }

    let peer_str = parts
        .get("peer")
        .copied()
        .ok_or_else(|| DiscoveryError("missing 'peer' field in TXT record".into()))?;

    peer_str
        .parse::<Multiaddr>()
        .map_err(|e| DiscoveryError(format!("invalid multiaddr '{peer_str}': {e}")))
}

/// Query DNS TXT records for bootstrap peers.
/// Record name: _wws._tcp.<domain>
pub async fn lookup_bootstrap_peers(domain: &str) -> Vec<Multiaddr> {
    use hickory_resolver::config::{ResolverConfig, ResolverOpts};
    use hickory_resolver::TokioAsyncResolver;

    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    let txt_name = format!("_wws._tcp.{domain}");
    match resolver.txt_lookup(&txt_name).await {
        Ok(records) => {
            let mut peers = Vec::new();
            for rdata in records.iter() {
                let rdata: &hickory_resolver::proto::rr::rdata::TXT = rdata;
                let strings: Vec<String> = rdata
                    .txt_data()
                    .iter()
                    .map(|chunk: &Box<[u8]>| String::from_utf8_lossy(chunk.as_ref()).to_string())
                    .collect();
                let combined = strings.join("");
                if let Ok(addr) = parse_bootstrap_txt_record(&combined) {
                    peers.push(addr);
                }
            }
            peers
        }
        Err(e) => {
            tracing::debug!("DNS bootstrap lookup failed for {txt_name}: {e}");
            vec![]
        }
    }
}
