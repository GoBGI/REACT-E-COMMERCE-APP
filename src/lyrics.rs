
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

async fn try_lyricwiki_lyrics(artist: &str, title: &str) -> Result<Option<Lyrics>, Error> {
    // Try exact page
    let url = &format!("https://lyrics.fandom.com/wiki/{}:{}", artist, title);
    debug!("fetching url {}", &url);
    if let Some(l) = parse_lyricwiki_lyrics(&reqwest::get(url).await?.text().await?) {
        return Ok(Some(Lyrics {
            lyrics: l,
            provider: "LyricWiki".to_owned(),
            source: url.to_string(),
        }));
    }

    // Fetch list of all pages associated with this artist
    let url = &format!(
        "http://lyrics.wikia.com/api.php?func=getArtist&artist={}&fmt=text",
        artist
    );
    debug!("fetching url {}", url);
    let song_list = reqwest::get(url).await?.text().await?;

    // Try to find the exact page name
    if let Some(t) = song_list.split('\n').find(|t| t.ends_with(title)) {
        let url = &format!("https://lyrics.fandom.com/wiki/{}", t);
        debug!("fetching url {}", url);
        if let Some(l) = parse_lyricwiki_lyrics(&reqwest::get(url).await?.text().await?) {
            return Ok(Some(Lyrics {
                lyrics: l,
                provider: "LyricWiki".to_owned(),
                source: url.to_string(),
            }));
        }
    }

    // Try to find the primary artist name and use it
    if let Some(artist) = song_list.split(':').next() {
        let url = &format!("https://lyrics.fandom.com/wiki/{}:{}", artist, title);
        debug!("fetching url {}", url);
        if let Some(l) = parse_lyricwiki_lyrics(&reqwest::get(url).await?.text().await?) {
            return Ok(Some(Lyrics {
                lyrics: l,
                provider: "LyricWiki".to_owned(),
                source: url.to_string(),
            }));
        }