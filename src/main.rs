mod discord;

use poise::serenity_prelude::GatewayIntents;
use poise::serenity_prelude;
use std::collections::HashMap;
use std::sync::Mutex;
use poise::samples::on_error;
use std::env::var;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use reqwest::header::AUTHORIZATION;
use serde_json::{to_string_pretty, Value};
use dotenv::dotenv;
use poise::FrameworkError;
use poise::serenity_prelude as serenity;

async fn fetch_recommendations() -> Result<Value, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let content = client
        .get("https://api.listenbrainz.org/1/user/magnias/playlists/createdfor")
        .header(AUTHORIZATION, format!("\"Authorization\": \"Token {}\"", var("LISTENBRAINZ_TOKEN").expect("LISTENBRAINZ_TOKEN should be set")))
        .send()
        .await?
        .text()
        .await?;

    let parsed: Value = serde_json::from_str(content.as_str())?;
    Ok(parsed)
}

fn get_weekly_exploration(recommendations: Value) -> Option<String> {
    let playlists = recommendations.get("playlists").expect("No playlists found");

    match playlists {
        Value::Array(list) => {
            for p in list {
                let source_patch = p.get("playlist")
                    .and_then(|u| u.get("extension"))
                    .and_then(|u| u.get("https://musicbrainz.org/doc/jspf#playlist"))
                    .and_then(|u| u.get("additional_metadata"))
                    .and_then(|u| u.get("algorithm_metadata"))
                    .and_then(|u| u.get("source_patch"))
                    .unwrap();
                if source_patch == "weekly-jams" {
                    let url = p.get("playlist")
                        .and_then(|u| u.get("identifier"))
                        .unwrap()
                        .to_string();
                    let mut mbid = url.trim_matches('"').split('/').last().unwrap();
                    return Some(mbid.to_string());
                }
            }
        },
        _ => todo!()
    }
    None
}

async fn add_albums(albums: &[Album]) -> Result<(), Box<dyn Error>> {
    for album in albums {
        println!("Adding album:");
        println!("id: {:?}", album.album_mbid);
        println!("title: {:?}", album.album);

        let client = reqwest::Client::new();
        let result = client
            .get(format!("http://music.nel.re:8181/api?apikey={}&cmd=addAlbum&id={}", var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set"), album.album_mbid))
            .send()
            .await?
            .text()
            .await?;
        println!("Added with status: {result}");
    }
    Ok(())
}

async fn fetch_playlist(mbid: &str) -> Result<Value, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let content = client
        .get(format!("https://api.listenbrainz.org/1/playlist/{mbid}"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: Value = serde_json::from_str(content.as_str())?;
    Ok(parsed)
}

struct Album
{
    album: String,
    album_mbid: String
}

async fn run_poise() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![discord::age()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(discord::Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    run_poise().await;

    return;

    let recommendations = fetch_recommendations().await.expect("Failed to query listenbrainz again");
    if let Some(playlist) = get_weekly_exploration(recommendations.clone()) {

        let content = fetch_playlist(&playlist).await.unwrap();

        match content.get("playlist").and_then(|u| u.get("track")).unwrap() {
            Value::Array(list) => {
                let albums = list.iter().filter_map(|entry| {
                    let pretty = to_string_pretty(entry).unwrap();
                    println!("{}", pretty);

                    if let Some(album_mbid) = entry.get("extension")
                        .and_then(|u| u.get("https://musicbrainz.org/doc/jspf#track"))
                        .and_then(|u| u.get("additional_metadata"))
                        .and_then(|u| u.get("caa_release_mbid")) {

                        let album_mbid_string = album_mbid.to_string()
                            .trim_matches('"')
                            .to_string();

                        let album = entry.get("album").unwrap()
                            .to_string()
                            .trim_matches('"')
                            .to_string();

                        return Some(Album {
                            album,
                            album_mbid: album_mbid_string,
                        })
                    } else {
                        println!("The listenbrainz metadata didn't contain an album id, skipping");
                        return None
                    }
                }).collect::<Vec<Album>>();

                let album_requests = albums.iter().map(|s| s.album_mbid.clone()).collect::<Vec<String>>();
                add_albums(&albums).await.unwrap();

            },
            _ => todo!()
        }
    }
}