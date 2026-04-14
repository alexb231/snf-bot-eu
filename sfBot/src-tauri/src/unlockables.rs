use std::{error::Error, time::Duration};

use sf_api::{command::Command, error::SFError, SimpleSession};
use tokio::time::sleep;

pub async fn sleep_between_commands(ms: u64) { sleep(Duration::from_millis(ms)).await; }

pub async fn accept_unlockables(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let unlocks = &gs.pending_unlocks.clone();

    if unlocks.len() > 0
    {
        for x in unlocks
        {
            let result = session.send_command(Command::UnlockFeature { unlockable: *x }).await;

            let cmd = match result
            {
                Ok(_) =>
                {
                    session.send_command(Command::Update).await?;
                    continue;
                }
                Err(SFError::ServerError(msg)) if msg == "feature is not unlockable" =>
                {
                    continue;
                }
                Err(e) =>
                {
                    continue;
                }
            };
        }
    }
    Ok(String::from(""))
}
