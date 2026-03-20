pub mod claude_code;
pub mod opencode;

use anyhow::Result;

use super::normalize::{ParsedSession, SessionRef};

pub trait SessionParser: Send + Sync {
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, session: &SessionRef) -> Result<ParsedSession>;
}
