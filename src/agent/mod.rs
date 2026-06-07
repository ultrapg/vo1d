pub mod loop_;
pub mod planner;
pub mod plan_parser;
pub mod session;
pub mod checkpoint;
pub mod parser;
pub mod executor;
pub mod train;

use crate::AppContext;
use crate::agent::session::Session;
use anyhow::Result;

/// Run the agent loop on a session until completion.
pub async fn run(ctx: AppContext, session: Session) -> Result<Session> {
    loop_::agent_loop(ctx, session).await
}
