use std::env::var;
use reqwest::header::AUTHORIZATION;
use serde_json::{Value};
use crate::error::Error;

#[derive(Debug)]
pub struct Album
{
    pub album: String,
    pub album_mbid: String
}

async fn fetch_playlist(mbid: &str) -> Result<Value, Error> {
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

pub async fn fetch_recommendations() -> Result<Value, Error> {
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
        _ => return None
    }
    None
}

pub async fn add_album(album: &Album) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let result = client
        .get(format!("http://music.nel.re:8181/api?apikey={}&cmd=addAlbum&id={}", var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set"), album.album_mbid))
        .send()
        .await?
        .text()
        .await?;
    Ok(())
}

pub async fn request_lb_recommended() -> Result<Vec<Album>, Error> {
    let recommendations = fetch_recommendations().await?;
    if let Some(playlist) = get_weekly_exploration(recommendations.clone()) {

        let content = fetch_playlist(&playlist).await?;

        match content.get("playlist").and_then(|u| u.get("track")).unwrap() {
            Value::Array(list) => {
                let albums = list.iter().filter_map(|entry| {
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

                return Ok(albums);
            },
            _ => return Err("Error parsing albums".into())
        }
    }
    Err("Error parsing albums".into())
}