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
}
