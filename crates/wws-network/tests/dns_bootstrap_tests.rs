#[test]
fn test_parse_dns_txt_record_valid() {
    let txt = "v=1 peer=/ip4/1.2.3.4/tcp/9000";
    let addr = wws_network::dns_bootstrap::parse_bootstrap_txt_record(txt).unwrap();
    assert!(addr.to_string().contains("1.2.3.4"));
}

#[test]
fn test_parse_dns_txt_record_invalid_no_peer() {
    let result = wws_network::dns_bootstrap::parse_bootstrap_txt_record("v=1 garbage");
    assert!(result.is_err());
}

#[test]
fn test_parse_dns_txt_record_wrong_version() {
    let txt = "v=2 peer=/ip4/1.2.3.4/tcp/9000";
    let result = wws_network::dns_bootstrap::parse_bootstrap_txt_record(txt);
    assert!(result.is_err());
}

#[test]
fn test_parse_dns_txt_record_missing_version() {
    let txt = "peer=/ip4/1.2.3.4/tcp/9000";
    let result = wws_network::dns_bootstrap::parse_bootstrap_txt_record(txt);
    assert!(result.is_err());
}
