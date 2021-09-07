mod config;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use serenity::{
    client::{Context, EventHandler},
    model::{
        id::ChannelId,
        interactions::{
            application_command::ApplicationCommand, message_component::ButtonStyle, Interaction,
            InteractionResponseType,
        },
    },
    Client,
};
use std::io;
use tokio::net::UdpSocket;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    setup_and_wait_for_discord().await;
}

static MAGIC_PACKET: Lazy<[u8; 102]> = Lazy::new(|| {
    let mut buf = [0xFFu8; 102];

    let addr: Vec<u8> = config::WOL_TARGET_DEVICE_MAC_ADDR
        .split("-")
        .map(|e| u8::from_str_radix(e, 16).unwrap())
        .collect();
    assert_eq!(addr.len(), 6);
    for i in 0..16 {
        buf[(6 + i * 6)..(12 + i * 6)].copy_from_slice(&addr);
    }
    buf
});

async fn post_wol() -> io::Result<()> {
    let sock = UdpSocket::bind("127.0.0.1:0").await?;
    sock.set_broadcast(true)?;
    sock.connect("255.255.255.255:0").await?;
    sock.send(&*MAGIC_PACKET).await?;
    Ok(())
}

async fn setup_and_wait_for_discord() {
    Client::builder(config::DISCORD_TOKEN)
        .application_id(config::DISCORD_APP_ID)
        .event_handler(Handler)
        .await
        .unwrap()
        .start()
        .await
        .unwrap();
}

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _data_about_bot: serenity::model::prelude::Ready) {
        ApplicationCommand::create_global_application_command(ctx.http, |c| {
            c.name("wol")
                .description("Init launch PC button")
                .default_permission(true)
        })
        .await
        .unwrap();
        println!("ready!");
    }

    async fn interaction_create(
        &self,
        ctx: Context,
        interaction: serenity::model::interactions::Interaction,
    ) {
        match interaction {
            Interaction::ApplicationCommand(init_command) => {
                if init_command.data.name == "wol" {
                    ChannelId(config::DISCORD_BUTTON_CHID)
                        .send_message(ctx.http, |m| {
                            m.content("Launch PC button");
                            m.components(|c| {
                                c.create_action_row(|r| {
                                    r.create_button(|b| {
                                        b.style(ButtonStyle::Primary)
                                            .custom_id("button_launch")
                                            .label("Launch!")
                                    })
                                })
                            })
                        })
                        .await
                        .unwrap();
                }
            }
            Interaction::MessageComponent(launch_wol_command) => {
                if launch_wol_command.data.custom_id == "button_launch" {
                    println!("launch!");
                    post_wol().await.unwrap();
                    launch_wol_command
                        .create_interaction_response(ctx.http, |r| {
                            r.kind(InteractionResponseType::DeferredUpdateMessage)
                        })
                        .await
                        .unwrap();
                }
            }
            Interaction::Ping(_) => todo!(),
        }
    }
}
