use std::cmp::Ordering;
use std::env::var;
use std::str::FromStr;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde_json::{Value};
use crate::error::Error;
use crate::music::AlbumStatus::{Downloaded, Nothing, Snatched, Wanted};

#[derive(Debug)]
pub struct Album
{
    pub album: String,
    pub artist: String,
    pub release_group: String,
    pub mbid: String,
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

    println!("Adding album ({} - {}) - {}", album.artist, album.album, album.mbid);
    println!("Release group {}", album.release_group);

    let client = reqwest::Client::new();

    // I need the following three request to load, fetch, and request albums.

    let result = client
        .get(format!("{}/api?apikey={}&cmd=getAlbum&id={}",
                     var("HEADPHONES_URI").expect("HEADPHONES_URI should be set"),
                     var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set"),
                     album.mbid)
        )
        .send()
        .await?
        .text()
        .await?;

    let result = client
        .get(format!("{}/api?apikey={}&cmd=addAlbum&id={}",
                     var("HEADPHONES_URI").expect("HEADPHONES_URI should be set"),
                     var("HEADPHONES_API_KEY").expect("HEADPHONES_API_KEY should be set"),
                     album.mbid)
        )
        .send()
        .await?
        .text()
        .await?;

    // Only do an add for now
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
        .as_array()
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("Status"))
        .and_then(|v| Some(v.to_string().trim_matches('"').to_string()))
        .unwrap_or("".to_string());

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
                        release_group,
                        mbid: album_mbid,
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

async fn get_albums(album_mbids: Vec<String>) -> Result<Vec<Album>, Error> {
    let client = reqwest::Client::new();
    let mbids_param = album_mbids.join("+");

    // Get the full album data with release groups
    let response = client
        .get(format!("https://musicbrainz.org/ws/2/release?query=mbid:({})&fmt=json", mbids_param))
        .header(USER_AGENT, format!("{}", "https://github.com/n-e-l/music-management (lauda@nel.re)"))
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("HTTP Error {}", response.status());
        return Err("Failed to fetch releases".into());
    }

    let content = response.text().await?;
    let parsed: Value = serde_json::from_str(&content)?;

    let mut results = Vec::new();
    if let Some(releases) = parsed.get("releases").and_then(|u| u.as_array()) {
        for release in releases {

            let rg = release.get("release-group").and_then(|u| u.get("id")).and_then(|u| u.as_str());
            let title = release.get("release-group").and_then(|u| u.get("title")).and_then(|u| u.as_str());
            let id = release.get("id").and_then(|u| u.as_str());
            let artist = release.get("artist-credit")
                .and_then(|u| u.as_array().unwrap().first())
                .and_then(|u| u.get("name")).and_then(|u| u.as_str());

            results.push(Album {
                album: title.unwrap().to_string(),
                artist: artist.unwrap().to_string(),
                mbid: id.unwrap().to_string(),
                release_group: rg.unwrap().to_string()
            });
        }
    }

    results.sort_by(|a, b|{
        if a.artist < b.artist {
            return Ordering::Less
        } else if a.artist > b.artist {
            return Ordering::Greater
        }

        if a.album < b.album {
            return Ordering::Less
        } else if a.album > b.album {
            return Ordering::Greater
        }

        Ordering::Equal
    });

    Ok(results)
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

    println!("Fetched release group info: {}", release_group);

    Ok(Some(release_group))
}

pub async fn lb_status() -> Result<Vec<(AlbumStatus, Album)>, Error> {

    let albums = request_lb_recommended().await?;

    let mut album_states = Vec::new();
    for album in albums {
        match album_info(&album).await {
            Ok(status) => {
                album_states.push((status, album));
            }
            Err(_) => {}
        }
    }

    Ok(album_states)
}

pub async fn request_lb_recommended() -> Result<Vec<Album>, Error> {
    let recommendations = fetch_recommendations().await?;
    if let Some(playlist) = get_weekly_exploration(recommendations.clone()) {

        let content = fetch_playlist(&playlist).await?;

        let album_mbids: Vec<String> = match content.get("playlist").and_then(|u| u.get("track")).unwrap() {
            Value::Array(list) => {
                list.iter().filter_map(|e| {

                    if let Some(album_mbid) = e.get("extension")
                        .and_then(|u| u.get("https://musicbrainz.org/doc/jspf#track"))
                        .and_then(|u| u.get("additional_metadata"))
                        .and_then(|u| u.get("caa_release_mbid")) {

                        let album_mbid_string = album_mbid.to_string()
                            .trim_matches('"')
                            .to_string();

                        Some(album_mbid_string)
                    } else {
                        println!("The listenbrainz metadata didn't contain an album id, skipping");
                        None
                    }
                }).collect::<Vec<String>>()
            },
            _ => vec![]
        };

        return Ok(get_albums(album_mbids).await?);

    }
    Err("Error parsing albums".into())
}