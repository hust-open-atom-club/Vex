use proptest::prelude::*;

use crate::config::{QemuConfig, parse_config_json, validate_config_name};
use crate::remote::RemoteSpec;

proptest! {
    #[test]
    fn remote_spec_parse_never_panics(input in "\\PC*") {
        let _ = RemoteSpec::parse(&input);
    }

    #[test]
    fn validate_config_name_never_panics(name in "\\PC*") {
        let _ = validate_config_name(&name);
    }

    #[test]
    fn valid_config_name_round_trips(name in "[a-zA-Z0-9._-]{1,255}") {
        if name != "." && name != ".." {
            validate_config_name(&name).unwrap();
        }
    }

    #[test]
    fn qemu_config_serde_round_trip(
        bin in "[a-z-]{1,30}",
        args in prop::collection::vec("[a-zA-Z0-9=-]{1,20}", 0..5),
        desc in proptest::option::of("[a-zA-Z ]{1,50}"),
        version in proptest::option::of("[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}"),
    ) {
        let config = QemuConfig {
            qemu_bin: bin,
            args,
            desc,
            qemu_version: version,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed = parse_config_json(&json).unwrap();
        assert_eq!(parsed.qemu_bin, config.qemu_bin);
        assert_eq!(parsed.args, config.args);
        assert_eq!(parsed.desc, config.desc);
        assert_eq!(parsed.qemu_version, config.qemu_version);
    }

    #[test]
    fn valid_remote_spec_round_trips(
        id in "[a-zA-Z0-9._-]{1,50}",
        name in "[a-zA-Z0-9._-]{1,50}",
        tag in proptest::option::of("[a-zA-Z0-9._-]{1,50}"),
    ) {
        if id != "." && id != ".." && name != "." && name != ".." {
            let input = match &tag {
                Some(t) if *t != "." && *t != ".." => format!("{}/{}:{}", id, name, t),
                None => format!("{}/{}", id, name),
                _ => return Ok(()),
            };
            let spec = RemoteSpec::parse(&input).unwrap();
            assert_eq!(spec.id, id);
            assert_eq!(spec.name, name);
            assert_eq!(spec.tag.as_deref(), tag.as_deref());
        }
    }

    #[test]
    fn substitute_params_never_panics(
        args in prop::collection::vec("\\PC{0,100}", 0..10),
    ) {
        use crate::commands::exec::substitute_params;
        let _ = substitute_params(&args, |_| None);
    }

    #[test]
    fn substitute_params_identity_no_vars(
        args in prop::collection::vec("[a-zA-Z0-9 =-]{0,30}", 0..10),
    ) {
        use crate::commands::exec::substitute_params;
        let result = substitute_params(&args, |_| None);
        assert_eq!(result, args);
    }

    #[test]
    fn parse_config_json_never_panics(input in "\\PC{0,500}") {
        let _ = parse_config_json(&input);
    }

    #[test]
    fn validate_config_never_panics(
        bin in "\\PC{0,50}",
        args in prop::collection::vec("\\PC{0,30}", 0..5),
        desc in proptest::option::of("\\PC{0,50}"),
    ) {
        use crate::config::validate_config;
        let config = QemuConfig {
            qemu_bin: bin,
            args,
            desc,
            qemu_version: None,
        };
        let _ = validate_config(&config);
    }

    #[test]
    fn published_config_serde_round_trip(
        id in "[a-zA-Z0-9._-]{1,30}",
        name in "[a-zA-Z0-9._-]{1,30}",
        tag in "[a-zA-Z0-9._-]{1,20}",
        bin in "[a-z-]{1,30}",
        args in prop::collection::vec("[a-zA-Z0-9=-]{1,20}", 0..5),
    ) {
        use crate::remote::PublishedConfig;
        if id != "." && id != ".." && name != "." && name != ".." && tag != "." && tag != ".." {
            let config = QemuConfig { qemu_bin: bin, args, desc: None, qemu_version: None };
            let published = PublishedConfig {
                schema_version: 1,
                id: id.clone(),
                name: name.clone(),
                tag: tag.clone(),
                config,
            };
            let json = serde_json::to_string(&published).unwrap();
            let parsed: PublishedConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.id, id);
            assert_eq!(parsed.name, name);
            assert_eq!(parsed.tag, tag);
            assert_eq!(parsed.schema_version, 1);
        }
    }

    #[test]
    fn config_name_rejected_always_contains_bad_char_or_structure(
        name in "\\PC{1,100}",
    ) {
        if validate_config_name(&name).is_err() {
            let is_dot_name = name == "." || name == "..";
            let too_long = name.len() > 255;
            let has_bad_char = !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
            let has_separator = name.contains('/') || name.contains('\\');
            let has_null = name.contains('\0');
            assert!(is_dot_name || too_long || has_bad_char || has_separator || has_null,
                "name '{}' was rejected but doesn't match any known rejection criterion", name);
        }
    }
}
