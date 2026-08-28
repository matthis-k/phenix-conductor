from pathlib import Path

path = Path("rust/crates/phenix-harness/src/runtime_config.rs")
text = path.read_text()
text = text.replace(
    "use std::{collections::BTreeMap, error::Error, fs, path::Path};",
    "use std::{collections::BTreeMap, error::Error, fs, io, path::Path};",
    1,
)
text = text.replace(
    "#[derive(Debug, Deserialize, Debug, Default, Deserialize)]",
    "#[derive(Debug, Default, Deserialize)]",
    1,
)
text = text.replace(
    "\nstruct RuntimeModelTarget {",
    "\n#[derive(Debug, Deserialize)]\nstruct RuntimeModelTarget {",
    1,
)
text = text.replace(
    "let session = OptionSubjectId::parse(session)?;",
    "let session = OptionSubjectId::parse(session)\n            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;",
    1,
)
text = text.replace(
    "let agent = OptionSubjectId::parse(agent)?;",
    "let agent = OptionSubjectId::parse(agent)\n            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;",
    1,
)
text = text.replace(
    "let key = OptionKey::parse(key)?;",
    "let key = OptionKey::parse(key)\n        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;",
    1,
)
path.write_text(text)
