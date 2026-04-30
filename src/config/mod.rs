pub mod builtins;
pub mod render;
pub mod schema;

pub use builtins::lookup;
pub use schema::{
    Config, FlexSegment, Line, Segment, TemplateSegment, ValidationError, ValidationWarning,
    MAX_LINES, MAX_PADDING,
};
