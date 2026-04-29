use crate::error::VexError;
use crate::remote::RemoteSpec;

#[test]
fn parse_valid_spec_with_tag() {
    let spec = RemoteSpec::parse("org/repo:v2").unwrap();
    assert_eq!(spec.id, "org");
    assert_eq!(spec.name, "repo");
    assert_eq!(spec.tag.as_deref(), Some("v2"));
}

#[test]
fn parse_valid_spec_without_tag() {
    let spec = RemoteSpec::parse("user/config").unwrap();
    assert_eq!(spec.id, "user");
    assert_eq!(spec.name, "config");
    assert!(spec.tag.is_none());
    assert_eq!(spec.resolved_tag(), "latest");
}

#[test]
fn parse_missing_slash_rejected() {
    let err = RemoteSpec::parse("noslash").unwrap_err();
    assert!(matches!(err, VexError::RemoteSpecInvalid { .. }));
}

#[test]
fn parse_empty_id_rejected() {
    let err = RemoteSpec::parse("/name").unwrap_err();
    assert!(matches!(err, VexError::RemoteSpecInvalid { .. }));
}

#[test]
fn parse_empty_name_rejected() {
    let err = RemoteSpec::parse("id/").unwrap_err();
    assert!(matches!(err, VexError::RemoteSpecInvalid { .. }));
}

#[test]
fn parse_dot_id_rejected() {
    assert!(RemoteSpec::parse("./name").is_err());
    assert!(RemoteSpec::parse("../name").is_err());
}

#[test]
fn parse_dot_name_rejected() {
    assert!(RemoteSpec::parse("id/.").is_err());
    assert!(RemoteSpec::parse("id/..").is_err());
}

#[test]
fn parse_path_traversal_in_id_rejected() {
    assert!(RemoteSpec::parse("../../etc/name").is_err());
}

#[test]
fn parse_special_chars_in_id_rejected() {
    assert!(RemoteSpec::parse("id with space/name").is_err());
    assert!(RemoteSpec::parse("id@host/name").is_err());
}

#[test]
fn parse_null_byte_in_segment_rejected() {
    assert!(RemoteSpec::parse("id\0evil/name").is_err());
    assert!(RemoteSpec::parse("id/name\0evil").is_err());
}

#[test]
fn parse_long_segment_rejected() {
    let long_id = "a".repeat(256);
    let input = format!("{}/name", long_id);
    assert!(RemoteSpec::parse(&input).is_err());
}

#[test]
fn parse_max_length_segment_accepted() {
    let id = "a".repeat(255);
    let input = format!("{}/name", id);
    assert!(RemoteSpec::parse(&input).is_ok());
}

#[test]
fn parse_valid_chars_accepted() {
    assert!(RemoteSpec::parse("my-org/my_config").is_ok());
    assert!(RemoteSpec::parse("org.name/repo.name").is_ok());
    assert!(RemoteSpec::parse("Org123/Config456").is_ok());
}
