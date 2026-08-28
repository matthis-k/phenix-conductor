from pathlib import Path

path = Path("rust/crates/phenix-harness/src/main.rs")
text = path.read_text()
old = '''        let precedence = match env::var("PHENIX_SETTINGS_PRECEDENCE").as_deref() {
            Ok("file") => OptionStartupPrecedence::File,
            Ok("nix") | Err(env::VarError::NotPresent) => OptionStartupPrecedence::Nix,
            Ok(value) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid PHENIX_SETTINGS_PRECEDENCE: {value}"),
                )
                .into())
            }
            Err(error) => return Err(error.into()),
        };
'''
new = '''        let precedence = match env::var("PHENIX_SETTINGS_PRECEDENCE") {
            Ok(value) if value == "file" => OptionStartupPrecedence::File,
            Ok(value) if value == "nix" => OptionStartupPrecedence::Nix,
            Err(env::VarError::NotPresent) => OptionStartupPrecedence::Nix,
            Ok(value) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid PHENIX_SETTINGS_PRECEDENCE: {value}"),
                )
                .into())
            }
            Err(error) => return Err(error.into()),
        };
'''
if old not in text:
    raise SystemExit("precedence match anchor missing")
path.write_text(text.replace(old, new, 1))

path = Path("rust/crates/phenix-harness/src/lib.rs")
text = path.read_text()
old = "        assert_eq!(harness.kernel().config().manifests().count(), 15);\n"
new = '''        assert_eq!(
            harness.kernel().config().manifests().count(),
            HarnessBuilder::default_suite_plugin_ids().len()
        );
'''
if old not in text:
    raise SystemExit("default suite count anchor missing")
path.write_text(text.replace(old, new, 1))

path = Path("rust/crates/phenix-harness/tests/supported_product_journeys.rs")
text = path.read_text()
old = '''    assert_eq!(
        harness.kernel().config().manifests().count(),
        15,
        "the supported Harness owns the complete first-party suite",
    );
'''
new = '''    assert_eq!(
        harness.kernel().config().manifests().count(),
        HarnessBuilder::default_suite_plugin_ids().len(),
        "the supported Harness owns the complete first-party suite",
    );
'''
if old not in text:
    raise SystemExit("supported suite count anchor missing")
path.write_text(text.replace(old, new, 1))
