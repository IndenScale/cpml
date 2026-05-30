use crate::error::CpmlError;
use crate::schema::CpmlDocument;

/// Parse a YAML string into a schema-level CpmlDocument.
pub fn parse_yaml(input: &str) -> Result<CpmlDocument, CpmlError> {
    let doc: CpmlDocument = serde_yaml::from_str(input)?;
    Ok(doc)
}
