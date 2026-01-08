use std::ops::{AddAssign};
use poise::CreateReply;
use poise::serenity_prelude::{CreateActionRow, CreateButton};
use crate::error::Error;
use crate::music;
use crate::music::{add_album, album_info, AlbumStatus};

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
        let add = format!("- {} - {}\n", a.artist, a.album);
       response.add_assign(add.as_str());
    });
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn lb_status(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let albums = music::lb_status().await?;

    let mut response = "".to_string();

    for (status, album) in albums {
        let checkmark = match status {
            AlbumStatus::Nothing => {
                ":red_circle:"
            }
            AlbumStatus::Wanted => {
                ":orange_circle:"
            }
            AlbumStatus::Snatched => {
                ":yellow_circle:"
            }
            AlbumStatus::Downloaded => {
                ":green_circle:"
            }
        };

        let extra = format!("- {} {} - {}\n", checkmark, album.artist, album.album);
        response.add_assign(&extra);
    }

    ctx.say(response).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn lb_import(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let mut response = "Adding albums\n".to_string();
    let reply = ctx.say(response.clone()).await?;

    let albums = music::request_lb_recommended().await?;

    for a in &albums {
        add_album(a).await?;

        let info = album_info(a).await?;
        let checkmark = match info {
            AlbumStatus::Nothing => {
                ":red_circle:"
            }
            AlbumStatus::Wanted => {
                ":orange_circle:"
            }
            AlbumStatus::Snatched => {
                ":yellow_circle:"
            }
            AlbumStatus::Downloaded => {
                ":green_circle:"
            }
        };

        // let url = format!("http://music.nel.re:8181/albumPage?AlbumID={}", a.release_group);
        // let extra = format!("- {} {} - [{}]({})\n", checkmark, a.artist, a.album, url);
        let extra = format!("- {} {} - {}\n", checkmark, a.artist, a.album);
        response.add_assign(&extra);

        reply.edit(
            ctx,
            poise::CreateReply::default()
                .content(response.clone())
        ).await?;
    }

    let extra = "done";
    response.add_assign(&extra);

    reply.edit(
        ctx,
        poise::CreateReply::default()
            .content(response.clone())
    ).await?;

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
    _ctx: Context<'_>,
    _album: String,
) -> Result<(), Error> {
    Ok(())
}
