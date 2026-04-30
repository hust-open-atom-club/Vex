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

#[test]
fn parse_empty_tag_rejected() {
    let err = RemoteSpec::parse("id/name:").unwrap_err();
    assert!(matches!(err, VexError::RemoteSpecInvalid { .. }));
}

#[test]
fn parse_multiple_colons_rejected() {
    let err = RemoteSpec::parse("id/name:v1:extra").unwrap_err();
    assert!(matches!(err, VexError::RemoteSpecInvalid { .. }));
}

#[test]
fn parse_multiple_slashes() {
    let spec = RemoteSpec::parse("id/name/extra");
    if let Ok(s) = spec {
        assert!(s.name.contains('/') || s.name == "name");
    }
}

#[test]
fn parse_just_slash_rejected() {
    let err = RemoteSpec::parse("/").unwrap_err();
    assert!(matches!(err, VexError::RemoteSpecInvalid { .. }));
}

#[test]
fn parse_tag_dot_rejected() {
    assert!(RemoteSpec::parse("id/name:.").is_err());
}

#[test]
fn parse_tag_dotdot_rejected() {
    assert!(RemoteSpec::parse("id/name:..").is_err());
}

#[test]
fn parse_tag_with_hyphens_and_dots() {
    let spec = RemoteSpec::parse("org/cfg:v1.2-rc1").unwrap();
    assert_eq!(spec.tag.as_deref(), Some("v1.2-rc1"));
}

#[test]
fn parse_long_tag_rejected() {
    let long_tag = "t".repeat(256);
    let input = format!("id/name:{}", long_tag);
    assert!(RemoteSpec::parse(&input).is_err());
}

#[test]
fn parse_tag_max_length_accepted() {
    let tag = "t".repeat(255);
    let input = format!("id/name:{}", tag);
    let spec = RemoteSpec::parse(&input).unwrap();
    assert_eq!(spec.tag.as_deref().unwrap().len(), 255);
}

#[test]
fn parse_both_segments_at_max_length() {
    let id = "a".repeat(255);
    let name = "b".repeat(255);
    let input = format!("{}/{}", id, name);
    let spec = RemoteSpec::parse(&input).unwrap();
    assert_eq!(spec.id.len(), 255);
    assert_eq!(spec.name.len(), 255);
}

#[test]
fn parse_unicode_in_id_rejected() {
    assert!(RemoteSpec::parse("团队/config").is_err());
}

#[test]
fn parse_unicode_in_name_rejected() {
    assert!(RemoteSpec::parse("team/配置").is_err());
}

#[test]
fn parse_unicode_in_tag_rejected() {
    assert!(RemoteSpec::parse("team/cfg:版本1").is_err());
}

#[test]
fn resolved_tag_returns_custom_tag() {
    let spec = RemoteSpec::parse("id/name:custom").unwrap();
    assert_eq!(spec.resolved_tag(), "custom");
}

#[test]
fn resolved_tag_returns_latest_when_none() {
    let spec = RemoteSpec::parse("id/name").unwrap();
    assert_eq!(spec.resolved_tag(), "latest");
}

#[test]
fn parse_spec_equality() {
    let a = RemoteSpec::parse("org/repo:v1").unwrap();
    let b = RemoteSpec::parse("org/repo:v1").unwrap();
    assert_eq!(a, b);

    let c = RemoteSpec::parse("org/repo:v2").unwrap();
    assert_ne!(a, c);
}

#[test]
fn parse_spec_clone() {
    let spec = RemoteSpec::parse("org/repo:v1").unwrap();
    let cloned = spec.clone();
    assert_eq!(spec, cloned);
}

#[test]
fn parse_whitespace_in_segments_rejected() {
    assert!(RemoteSpec::parse("id with space/name").is_err());
    assert!(RemoteSpec::parse("id/name with space").is_err());
    assert!(RemoteSpec::parse("id/name:tag with space").is_err());
}

#[test]
fn parse_null_byte_in_tag_rejected() {
    assert!(RemoteSpec::parse("id/name:tag\0evil").is_err());
}
