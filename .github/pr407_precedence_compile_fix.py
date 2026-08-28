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
