use std::fmt;

#[derive(Debug)]
pub enum SchemaError {
    Located {
        location: String,
        source: Box<SchemaError>,
    },
    InvalidJson(serde_json::Error),
    InvalidRoot,
    InvalidDialect,
    InvalidDialectLocation,
    UnsupportedDialect(String),
    InvalidType(String),
    InvalidProperties,
    InvalidDefinitions,
    InvalidRequired,
    InvalidPropertySchema(String),
    InvalidDefinitionSchema(String),
    InvalidKeywordValue(&'static str),
}

impl SchemaError {
    pub fn at(self, location: &str) -> Self {
        match self {
            Self::Located { .. } | Self::InvalidJson(_) => self,
            source => Self::Located {
                location: location.to_owned(),
                source: Box::new(source),
            },
        }
    }

    pub fn is_invalid_root(&self) -> bool {
        match self {
            Self::Located { source, .. } => source.is_invalid_root(),
            Self::InvalidRoot => true,
            _ => false,
        }
    }
}

#[rustfmt::skip]
impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Located { location, source }        => write!(formatter, "at {location}: {source}"),
            Self::InvalidJson(error)                  => write!(formatter, "invalid JSON: {error}"),
            Self::InvalidRoot                         => write!(formatter, "schema root must be an object or boolean"),
            Self::InvalidDialect                      => write!(formatter, "'$schema' must be a string"),
            Self::InvalidDialectLocation              => write!(formatter, "'$schema' requires a schema resource root"),
            Self::UnsupportedDialect(uri)             => write!(formatter, "unsupported dialect: {uri}"),
            Self::InvalidType(kind)                   => write!(formatter, "invalid schema type: {kind}"),
            Self::InvalidProperties                   => write!(formatter, "'properties' must be an object"),
            Self::InvalidDefinitions                  => write!(formatter, "'$defs' must be an object"),
            Self::InvalidRequired                     => write!(formatter, "'required' must contain unique strings"),
            Self::InvalidPropertySchema(name)         => write!(formatter, "property {name:?} must contain a schema object or boolean"),
            Self::InvalidDefinitionSchema(name)       => write!(formatter, "definition {name:?} must contain a schema object or boolean"),
            Self::InvalidKeywordValue(keyword)        => write!(formatter, "invalid '{keyword}' value"),
        }
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Located { source, .. } => Some(source),
            Self::InvalidJson(error) => Some(error),
            _ => None,
        }
    }
}
