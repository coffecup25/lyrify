mod authorizer;
mod response;

use authorizer::Authorizer;
use serde_json::json;
use soup::prelude::*;
use std::{env, error::Error, fs, io::{self, Read, Stderr}, marker::PhantomData, str::FromStr};
use response::{GeniusAuth, Response,SpotifyAuth};



fn main() -> Result<(), Box<dyn Error>> {

    let (spotify_auth,genius_auth)=setup();

    /* We don't access anything within the user scopes so we don't need to authorize the app for the user
    let genius_auth = Authorizer::<GeniusAuthResponse>::from_env();
    let genius_auth_response = genius_auth.authorize();
     */

    // this cookie is super important. without it genius might return one of two different page layouts for the lyrics which makes scraping much harder. The page layout changes att the cookie value 50.
    let cookie_jar = reqwest::cookie::Jar::default();
    cookie_jar.add_cookie_str(
        "_genius_ab_test_cohort=80",
        &"https://genius.com".parse::<reqwest::Url>().unwrap(),
    );

    let client = reqwest::blocking::Client::builder()
        .cookie_provider(cookie_jar.into())
        .build()
        .unwrap();

    println!("Ready for queries");
    let mut buf = String::new();
    let input = io::stdin();
    loop {
        println!("Hit enter to get lyrics");
        input.read_line(&mut buf).unwrap();
        match get_lyrics(&client, &spotify_auth, &genius_auth) {
            Ok(lyrics) => println!("###########################\n{}\n", lyrics.trim_end()),
            Err(_) => println!("Couldn't find lyrics"),
        }
    }

    Ok(())
}

fn setup() -> (SpotifyAuth,GeniusAuth){
    
    let file_path= String::from("./config.json");
    let authorizer= Authorizer::<SpotifyAuth>::from_json_file(&file_path);
    //let authorizer= Authorizer::<SpotifyAuth>::from_env();

    let spotify_auth=authorizer.authorize();

    let authorizer= Authorizer::<GeniusAuth>::from_json_file(&file_path);

    let genius_auth=authorizer.authorize();
    // if let Some(refresh_token) = spotify_config.get("REFRESH_TOKEN"){
        
    // }else{
    //     let spotify_auth = Authorizer::<SpotifyAuth>::from_env();
    //     let spotify_auth_response = spotify_auth.authorize();
    // } 
    
    (spotify_auth,genius_auth)
}

fn get_lyrics(
    client: &reqwest::blocking::Client,
    spotify_auth: &SpotifyAuth,
    genius_auth: &GeniusAuth,
) -> Result<String, ()> {
    let spotify_response = spotify_auth
        .query("https://api.spotify.com/v1/me/player/currently-playing",client)
        .unwrap();

    let mut song = spotify_response["item"]["name"].to_string();
    let artist = &spotify_response["item"]["artists"][0]["name"];

    #[cfg(feature = "debug")]
    std::fs::write("response_spotify.json", &spotify_response.to_string())
        .expect("lord we fucked up");

    println!("The currently playing track is: {} by {}", song, artist);

    song = remove_feat(&mut song);

    let query_url = reqwest::Url::parse_with_params(
        "https://api.genius.com/search",
        &[("q", format!("{} {}", &song, artist))],
    )
    .unwrap();

    let genius_response = match genius_auth.query(&query_url.to_string(),client) {
        Ok(json_result) => json_result,
        Err(s) => panic!("{}", s),
    };

    #[cfg(feature = "debug")]
    std::fs::write("response_genius.json", &genius_response.to_string())
        .expect("lord we fucked up");

    let _song = &genius_response["response"]["hits"][0]["result"]["id"];
    let lyric_path = &genius_response["response"]["hits"][0]["result"]["path"];

    let query_url = reqwest::Url::from_str(&format!(
        "https://genius.com{}",
        lyric_path.as_str().unwrap()
    ))
    .unwrap();
    println!("{}", &query_url);

    let response = client.get(query_url).send().unwrap();

    #[cfg(feature = "debug")]
    println!(
        "############# Response #############\n {:?}",
        response.headers()
    );

    let res = response.text().unwrap();

    //println!("\n\n{:#?}",response.text().unwrap());

    #[cfg(feature = "debug")]
    std::fs::write("response_final.html", &res.as_bytes()).expect("lord we fucked up");

    //tag("div").attr("class", "lyrics")
    let document = soup::Soup::new(res.as_str());
    match document.tag("div").class("lyrics").find() {
        Some(n) => Ok(n.text()),
        None => Err(()),
    }
}


struct GeniusHits {
    hits: serde_json::Value,
    counter: usize,
}

struct Hit {
    song: String,
    artist: String,
    lyrics_path: String,
    song_art_path: String,
}

impl Iterator for GeniusHits {
    type Item = serde_json::Value;

    fn next(&mut self) -> Option<Self::Item> {
        let song = &self.hits[self.counter]["result"]["id"];
        let lyric_path = &self.hits[self.counter]["result"]["path"];
        todo!()
    }
}

#[derive(Clone, Copy)]
struct Encloser(char, char);

fn remove_feat(name: &mut String) -> String {
    let enclosers = [Encloser('(', ')'), Encloser('[', ']')];
    let mut enumerator = name.split_whitespace();
    let mut new_string = String::with_capacity(name.len());

    while let Some(word) = enumerator.next() {
        let mut word_iter = word.chars();
        if let Some(c) = word_iter.next() {
            // first character of word should be an encloser
            if let Some(encloser) = enclosers.iter().find(|&x| c == x.0) {
                if word_iter.as_str().to_lowercase() == "feat." {
                    while let Some(w) = enumerator.next() {
                        if w.chars().last().unwrap() == encloser.1 {
                            break;
                        }
                    }
                } else {
                    new_string.push_str(word);
                    new_string.push_str(" ");
                }
            } else {
                new_string.push_str(word);
                new_string.push_str(" ");
            }
        }
    }
    new_string = new_string.trim().to_string();
    new_string
}


#[cfg(test)]
mod tests {
    use crate::remove_feat;

    #[test]
    fn test_remove_feature_in_song_name() {
        let mut song_name = String::from("Skrawberries (feat. BJ The Chicago Kid)");
        let cleared_string = remove_feat(&mut song_name);
        let correct_string = String::from("Skrawberries");

        assert_eq!(correct_string, cleared_string);

        let mut song_name = String::from("CASH MANIAC | CAZH MAN1AC [FEAT. NYYJERYA]");
        let cleared_string = remove_feat(&mut song_name);
        let correct_string = String::from("CASH MANIAC | CAZH MAN1AC");

        assert_eq!(correct_string, cleared_string);

        let mut song_name = String::from("Beauty In The Dark (Groove With You)");
        let cleared_string = remove_feat(&mut song_name);
        let correct_string = String::from("Beauty In The Dark (Groove With You)");

        assert_eq!(correct_string, cleared_string);
    }
}
