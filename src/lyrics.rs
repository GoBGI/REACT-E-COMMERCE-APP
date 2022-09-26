
use std::result::Result;

use reqwest::Error;

#[derive(Debug)]
pub struct Lyrics {
    pub lyrics: String,
    pub provider: String,
    pub source: String,
}

pub async fn try_fetch_lyrics(artist: &str, title: &str) -> Result<Option<Lyrics>, Error> {
    Ok(try_lyricwiki_lyrics(artist, title).await?)
}
