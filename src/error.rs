use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A top-level line that is neither `:section`, `key=value`, nor blank.
    UnexpectedLine,
    /// An entry line without a `:` separator.
    MissingColon,
    /// An entry with nothing before the `:`.
    EmptyKey,
    /// A `}` that does not close any open map.
    UnexpectedCloseBrace,
    /// A `:section` that never reached its `:end`.
    UnterminatedSection,
    /// A `{` that never reached its `}`.
    UnterminatedMap,
    /// Maps nested deeper than the built-in limit.
    TooDeep,
}

impl ParseErrorKind {
    fn message(self) -> &'static str {
        match self {
            Self::UnexpectedLine => "unexpected line outside of a section",
            Self::MissingColon => "entry is missing ':'",
            Self::EmptyKey => "entry has an empty key",
            Self::UnexpectedCloseBrace => "'}' without a matching '{'",
            Self::UnterminatedSection => "section is missing ':end'",
            Self::UnterminatedMap => "map is missing '}'",
            Self::TooDeep => "maps are nested too deeply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

impl ParseError {
    pub(crate) fn new(line: usize, kind: ParseErrorKind) -> Self {
        Self { line, kind }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.kind.message())
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidData(pub &'static str);

impl fmt::Display for InvalidData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid probe data: {}", self.0)
    }
}

impl std::error::Error for InvalidData {}
