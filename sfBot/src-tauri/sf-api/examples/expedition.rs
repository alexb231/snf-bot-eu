use std::{borrow::Borrow, time::Duration};

use chrono::{DateTime, Local};
use sf_api::{
    command::{Command, ExpeditionSetting, TimeSkip},
    gamestate::tavern::{AvailableTasks, ExpeditionStage},
    session::SimpleSession,
};
use tokio::time::sleep;

#[tokio::main]
pub async fn main() {
    let mut session = login_with_env().await;

    loop {
        let gs = session.game_state().unwrap();
        let exp = &gs.tavern.expeditions;

        let Some(active) = exp.active() else {
            
            
            if !gs.tavern.is_idle() {
                println!(
                    "Waiting/Collection other actions is not part of this \
                     example"
                );
                break;
            }

            let expeditions = match gs.tavern.available_tasks() {
                AvailableTasks::Quests(_) => {
                    
                    
                    if !exp.is_event_ongoing() {
                        println!(
                            "Expeditions are currently not enabled, so we can \
                             not do anything"
                        );
                        break;
                    }
                    if gs.tavern.questing_preference
                        == ExpeditionSetting::PreferQuests
                    {
                        
                        
                        if !gs.tavern.can_change_questing_preference() {
                            println!(
                                "Expeditions are disabled in the settings and \
                                 that setting can not be changed today"
                            );
                            break;
                        }
                        println!("Changing expedition setting");
                        session
                            .send_command(
                                Command::SetQuestsInsteadOfExpeditions {
                                    value: ExpeditionSetting::PreferExpeditions,
                                },
                            )
                            .await
                            .unwrap();
                        continue;
                    }
                    println!("There seem to be no expeditions");
                    break;
                }
                AvailableTasks::Expeditions(expeditions) => expeditions,
            };

            
            
            let target = expeditions.first().unwrap();

            
            
            if target.thirst_for_adventure_sec
                > gs.tavern.thirst_for_adventure_sec
            {
                
                
                println!("We do not have enough thirst for adventure left");
                break;
            }

            
            println!("Starting expedition");
            session
                .send_command(Command::ExpeditionStart { pos: 0 })
                .await
                .unwrap();
            continue;
        };
        let current = active.current_stage();

        let cmd = match current {
            ExpeditionStage::Boss(_) => {
                println!("Fighting the expedition boss");
                Command::ExpeditionContinue
            }
            ExpeditionStage::Rewards(rewards) => {
                if rewards.is_empty() {
                    panic!("No rewards to choose from");
                }
                println!("Picking reward");
                
                Command::ExpeditionPickReward { pos: 0 }
            }
            ExpeditionStage::Encounters(roads) => {
                if roads.is_empty() {
                    panic!("No crossroads to choose from");
                }
                
                println!("Choosing crossroad");
                Command::ExpeditionPickEncounter { pos: 0 }
            }
            ExpeditionStage::Finished => {
                
                
                continue;
            }
            ExpeditionStage::Waiting { busy_until, .. } => {
                let remaining = time_remaining(busy_until);
                if remaining.as_secs() > 60 && gs.tavern.quicksand_glasses > 0 {
                    println!("Skipping the {}s wait", remaining.as_secs());
                    Command::ExpeditionSkipWait {
                        typ: TimeSkip::Glass,
                    }
                } else {
                    println!(
                        "Waiting {}s until next expedition step",
                        remaining.as_secs(),
                    );
                    sleep(remaining).await;
                    Command::Update
                }
            }
            ExpeditionStage::Unknown => panic!("unknown expedition stage"),
        };
        sleep(Duration::from_secs(1)).await;
        session.send_command(cmd).await.unwrap();
    }
}

pub fn time_remaining<T: Borrow<DateTime<Local>>>(time: T) -> Duration {
    (*time.borrow() - Local::now()).to_std().unwrap_or_default()
}

pub async fn login_with_env() -> SimpleSession {
    let username = std::env::var("USERNAME").unwrap();
    let password = std::env::var("PASSWORD").unwrap();
    let server = std::env::var("SERVER").unwrap();
    SimpleSession::login(&username, &password, &server)
        .await
        .unwrap()
}
