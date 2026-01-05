use std::fs::read;
use std::ops::{Add, AddAssign};
use crate::error::Error;
use crate::music;
use crate::music::add_album;

pub struct Data {} // User data, which is stored and accessible in all command invocations
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(track_edits, slash_command, prefix_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is an example bot made to showcase features of my custom Discord bot framework",
            ..Default::default()
        },
    )
        .await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn lb_recommends(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let albums = music::request_lb_recommended().await?;
    let mut response = "".to_string();
    albums.iter().for_each(|a| {
       response.add_assign(&*("- ".to_owned() + &a.album + "\n"));
    });
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn lb_import(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let albums = music::request_lb_recommended().await?;

    let mut response = "Adding albums\n".to_string();
    let reply = ctx.say(response.clone()).await?;

    for a in &albums {
        add_album(a).await?;

        let extra = format!("- {}\n", a.album);
        response.add_assign(&extra);

        reply.edit(
            ctx,
            poise::CreateReply::default()
                .content(response.clone())
        ).await?;
    }

    Ok(())
}
