use sf_api::{
    command::Command,
    gamestate::social::{ClaimableMailType, ClaimableStatus},
    SimpleSession,
};

pub async fn collect_gifts_from_mail(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let mut msg = String::from("");
    let free_slots = &gs.character.inventory.count_free_slots();
    if (free_slots <= &0)
    {
        return Ok(msg);
    }

    for claimable in &gs.mail.claimables
    {
        if claimable.status != ClaimableStatus::Claimed && (claimable.typ == ClaimableMailType::TwitchDrop || claimable.typ == ClaimableMailType::Coupon)
        {
            let gs = session.send_command(Command::Update).await?;
            if (gs.character.inventory.count_free_slots() <= 0)
            {
                return Ok(String::from(""));
            }

            let command = match claimable.typ
            {
                ClaimableMailType::TwitchDrop => Command::ClaimableClaim { msg_id: claimable.msg_id },
                ClaimableMailType::Coupon => Command::ClaimableClaim { msg_id: claimable.msg_id },
                ClaimableMailType::SupermanDelivery => Command::ClaimableClaim { msg_id: claimable.msg_id },
                ClaimableMailType::GenericDelivery => todo!(),
            };
            msg += "Collected gift from mail \n";
            session.send_command(command).await?;
        }
    }
    Ok(msg)
}
