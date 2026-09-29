//! The typed reference `ekr resolve` reads: a node named by its type and its aliases, never by
//! its canonical name (`docs/cli.md`, "`ekr resolve`").

use ekr_core::TypeId;
use serde::{Deserialize, Serialize};

use super::DocumentError;

/// A `typed-reference` document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedReference {
    /// A concrete node type, never an abstract one or one with a subtype.
    pub type_id: TypeId,
    /// The names the node is known by, each compared byte for byte.
    pub aliases: Vec<String>,
}

impl TypedReference {
    /// A node of `type_id` known by `aliases`.
    #[must_use]
    pub fn new<S: Into<String>>(type_id: TypeId, aliases: impl IntoIterator<Item = S>) -> Self {
        Self {
            type_id,
            aliases: aliases.into_iter().map(Into::into).collect(),
        }
    }

    /// The document as YAML. A typed reference holds no tag, and every alias is written as a
    /// string, quoted where YAML would read it as something else.
    ///
    /// # Errors
    /// [`DocumentError::Yaml`] when the writer refuses a value.
    pub fn to_yaml(&self) -> Result<String, DocumentError> {
        super::to_yaml(self)
    }
}
