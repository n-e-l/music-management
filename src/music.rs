use std::env::var;
use std::fmt::format;
use std::str::FromStr;
use poise::serenity_prelude::json::to_string_pretty;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde_json::{json, Value};
use crate::error::Error;
use crate::music::AlbumStatus::{Downloaded, Nothing, Snatched, Wanted};

#[derive(Debug)]
pub struct Album
{
    pub album: String,
    pub artist: String,
    pub album_mbid: String,
    pub release_group: String,
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
        .header(USER_AGENT, format!("{}", "https://github.com/n-e-l/music-management (lauda@nel.re)"))
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
                if source_patch == "weekly-exploration" {
                    let url = p.get("playlist")
                        .and_then(|u| u.get("identifier"))
                        .unwrap()
                        .to_string();
                    let mbid = url.trim_matches('"').split('/').last().unwrap();
                    return Some(mbid.to_string());
                }
            }
        },
        _ => return None
    }
    None
}

pub struct AlbumState {
    pub status: String,
    pub title: String
}

pub async fn headphones_status() -> Result<Vec<AlbumState>, Error> {
    let client = reqwest::Client::new();
    let result = client
        .get(format!("{}/api?apikey={}&cmd=getHistory",
                     var("HEADPHONES_URI").expect("HEADPHONES_URI should be set"),
                     var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set")
        )
        )
        .send()
        .await?
        .text()
        .await?;

    let json = Value::from_str(&result)?;
    let album_states = json.as_array().unwrap().iter().map(|entry| {
        let title = entry.get("Title").unwrap().clone().to_string();
        let status = entry.get("Status").unwrap().clone().to_string();

        AlbumState {
            title,
            status
        }
    }).collect();

    Ok(album_states)
}

pub async fn add_album(album: &Album) -> Result<String, Error> {
    let client = reqwest::Client::new();
    client
        .get(format!("{}/api?apikey={}&cmd=addAlbum&id={}",
                     var("HEADPHONES_URI").expect("HEADPHONES_URI should be set"),
                     var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set"),
                     album.release_group)
        )
        .send()
        .await?
        .text()
        .await?;

    let result = client
        .get(format!("{}/api?apikey={}&cmd=queueAlbum&id={}",
                     var("HEADPHONES_URI").expect("HEADPHONES_URI should be set"),
                     var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set"),
                     album.release_group)
        )
        .send()
        .await?
        .text()
        .await?;
    Ok(result)
}

#[derive(Debug)]
pub enum AlbumStatus {
    Nothing,
    Wanted,
    Snatched,
    Downloaded
}

pub async fn album_info(album: &Album) -> Result<AlbumStatus, Error> {
    let client = reqwest::Client::new();
    let result = client
        .get(format!("{}/api?apikey={}&cmd=getAlbum&id={}",
                     var("HEADPHONES_URI").expect("HEADPHONES_URI should be set"),
                     var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set"),
                     album.release_group)
        )
        .send()
        .await?
        .text()
        .await?;

    let value: Value = serde_json::from_str(result.as_str())?;

    let album_json = value.get("album").unwrap();
    let status = album_json
        .as_array().unwrap()
        .get(0).unwrap()
        .get("Status").unwrap_or(&Value::Null)
        .to_string()
        .trim_matches('"')
        .to_string();

    if status == "Downloaded" {
        return Ok(Downloaded);
    }
    if status == "Wanted" {
        return Ok(Wanted);
    }
    if status == "Snatched" {
        return Ok(Snatched);
    }

    Ok(Nothing)
}

pub async fn lb_search_album(
    album_title: Option<String>,
    artist_name: Option<String>
) -> Result<Vec<Album>, Error> {

    let query = if album_title.is_some() && artist_name.is_some() {
        format!(
            "release:{} AND artist:{}",
            album_title.as_ref().unwrap(),
            artist_name.as_ref().unwrap()
        )
    } else if album_title.is_some() {
        format!(
            "release:{}",
            album_title.as_ref().unwrap()
        )
    } else if artist_name.is_some() {
        format!(
            "artist:{}",
            artist_name.as_ref().unwrap()
        )
    } else {
        return Err("Didn't provide any search parameters".into());
    };
    println!("{}", query);

    let url = format!(
        "https://musicbrainz.org/ws/2/release/?query={}&fmt=json",
        urlencoding::encode(&query)
    );

    let client = reqwest::Client::new();
    let content = client
        .get(url)
        .header(AUTHORIZATION, format!("\"Authorization\": \"Token {}\"", var("LISTENBRAINZ_TOKEN").expect("LISTENBRAINZ_TOKEN should be set")))
        .header(USER_AGENT, format!("{}", "https://github.com/n-e-l/music-management (lauda@nel.re)"))
        .send()
        .await?
        .text()
        .await?;

    let mut albums = vec![];

    let value: Value = serde_json::from_str(content.as_str())?;
    match value.get("releases").unwrap() {
        Value::Array(list) => {
            for p in list {
                let pretty = to_string_pretty(&p).unwrap();
                println!("{}", pretty);

                let artist = p.get("artist-credit")
                    // TODO: Get all artists
                    .and_then(|u| u.as_array().unwrap().first())
                    .and_then(|u| u.get("artist"))
                    .and_then(|u| u.get("name"))
                    .unwrap()
                    .to_string();

                let title = p.get("title")
                    .unwrap()
                    .to_string();

                let album_mbid = p.get("id")
                    .unwrap()
                    .to_string();

                if let Some(release_group) = get_release_group(album_mbid.clone()).await? {
                    let album = Album {
                        album: title,
                        album_mbid,
                        release_group,
                        artist
                    };
                    albums.push(album);
                }
            }
        },
        _ => {}
    }

    Ok(albums)
}

async fn get_release_group(mbid: String) -> Result<Option<String>, Error> {
    let client = reqwest::Client::new();
    let content = client
        .get(format!("https://musicbrainz.org/ws/2/release/{}?inc=release-groups&fmt=json", mbid))
        .header(USER_AGENT, format!("{}", "https://github.com/n-e-l/music-management (lauda@nel.re)"))
        .send()
        .await?
        .text()
        .await?;

    let parsed: Value = serde_json::from_str(content.as_str())?;

    if parsed.get("release-group").is_none() {
        return Ok(None);
    }

    let release_group = parsed
        .get("release-group").unwrap()
        .get("id").unwrap()
        .to_string()
        .trim_matches('"')
        .to_string();

    Ok(Some(release_group))
}

pub async fn request_lb_recommended() -> Result<Vec<Album>, Error> {
    let recommendations = fetch_recommendations().await?;
    if let Some(playlist) = get_weekly_exploration(recommendations.clone()) {

        let content = fetch_playlist(&playlist).await?;

        match content.get("playlist").and_then(|u| u.get("track")).unwrap() {
            Value::Array(list) => {

                let mut albums = Vec::new();
                for entry in list {

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

                        let artist = "".to_string();

                        if let Some(release_group) = get_release_group(album_mbid_string.clone()).await? {
                            albums.push(Album {
                                album,
                                album_mbid: album_mbid_string,
                                release_group,
                                artist
                            });
                        }
                    } else {
                        println!("The listenbrainz metadata didn't contain an album id, skipping");
                    }
                }

                return Ok(albums);
            },
            _ => return Err("Error parsing albums".into())
        }
    }
    Err("Error parsing albums".into())
}