use std::fmt::format;
use std::ops::{AddAssign};
use std::panic::resume_unwind;
use poise::CreateReply;
use poise::serenity_prelude::{CreateActionRow, CreateButton};
use poise::serenity_prelude::json::to_string_pretty;
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

#[poise::command(slash_command, prefix_command)]
pub async fn album(
    ctx: Context<'_>,
    #[description = "Specific album to search"]
    album: String,
) -> Result<(), Error> {

    let result = music::lb_search_album(Some(album), None).await?;

    let mut response = "Search results\n".to_string();
    let mut buttons = Vec::new();
    for (i, r) in result.iter().enumerate() {
        let album = format!("{}. {} - {}\n", i, r.artist, r.album);
        response.add_assign(album.as_str());

        buttons.push( CreateButton::new(i.to_string()).label(format!("{}", i)) );
    }

    let mut components = vec![];
    let rows = buttons.len() / 5;
    for i in 0..rows {
        let mut row_buttons = Vec::new();
        for bi in 0..5 {
            let index = i * 5 + bi;
            if index < buttons.len() {
                row_buttons.push(buttons[index].clone());
            }
        }
        components.push(CreateActionRow::Buttons(row_buttons));
    }

    ctx.send(
        CreateReply::default()
            .content(response)
            .components(components)
    ).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn artist(
    ctx: Context<'_>,
    #[description = "Specific artist to search"]
    artist: String,
) -> Result<(), Error> {

    let result = music::lb_search_album(None, Some(artist)).await?;

    let mut response = "Search results\n".to_string();
    let mut buttons = Vec::new();
    for (i, r) in result.iter().enumerate() {
        let album = format!("{}. {} - {}\n", i, r.artist, r.album);
        response.add_assign(album.as_str());

        buttons.push( CreateButton::new(i.to_string()).label(format!("{}", i)) );
    }

    let mut components = vec![];
    let rows = buttons.len() / 5;
    for i in 0..rows {
        let mut row_buttons = Vec::new();
        for bi in 0..5 {
            let index = i * 5 + bi;
            if index < buttons.len() {
                row_buttons.push(buttons[index].clone());
            }
        }
        components.push(CreateActionRow::Buttons(row_buttons));
    }

    ctx.send(
        CreateReply::default()
            .content(response)
            .components(components)
    ).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, subcommands("artist", "album"))]
pub async fn search(
    ctx: Context<'_>,
    album: String,
) -> Result<(), Error> {
    Ok(())
}
