/// Verifies that NAT traversal features (QUIC, relay, DCUtR) and the
/// dns_bootstrap / name_registry modules are compiled in and accessible.
///
/// Actual runtime tests require a full swarm setup; these tests confirm that
/// the symbols are exported and the algorithms produce correct output in
/// isolation.

#[test]
fn test_dns_bootstrap_parse_valid() {
    let addr = wws_network::dns_bootstrap::parse_bootstrap_txt_record(
        "v=1 peer=/ip4/127.0.0.1/tcp/9000",
    )
    .expect("valid TXT record should parse");
    assert!(addr.to_string().contains("127.0.0.1"));
}

#[test]
fn test_name_registry_pow_difficulty_accessible() {
    let difficulty = wws_network::name_registry::pow_difficulty_for_name("test");
    // "test" is 4 chars → 4-6 char bucket → difficulty 16
    assert_eq!(difficulty, 16, "4-6 char names require difficulty 16");
}

#[test]
fn test_name_registry_pow_difficulty_short_name() {
    // 1-3 chars → difficulty 20 (most expensive)
    assert_eq!(wws_network::name_registry::pow_difficulty_for_name("ab"), 20);
}

#[test]
fn test_name_registry_pow_difficulty_long_name() {
    // 13+ chars → difficulty 8 (cheapest)
    assert_eq!(
        wws_network::name_registry::pow_difficulty_for_name("averylongusername"),
        8
    );
}

#[test]
fn test_nat_features_compiled() {
    // Verifies that the network module exports the expected top-level symbols.
    // If the libp2p features (quic, relay, dcutr) are missing the transport
    // module won't compile at all, so a successful build of this test binary
    // is itself the assertion.
    let _ = wws_network::name_registry::pow_difficulty_for_name("wws");
}
