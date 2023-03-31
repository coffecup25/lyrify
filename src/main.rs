mod authorizer;
mod response;
mod text;

use authorizer::Authorizer;

use response::genius_response::GeniusClient;
use response::spotify_response::SpotifyClient;
use std::{io, str::FromStr};

use crate::text::remove_feat;

fn main() {
    let (spotify_auth, genius_auth) = setup();
    let client = setup_http_client();
    println!("Ready for queries");
    run_lyrify(&spotify_auth, &genius_auth, &client)
}

fn run_lyrify(spotify_auth: &SpotifyClient, genius_auth: &GeniusClient, client: &reqwest::blocking::Client) {
    let mut buf = String::new();
    let input = io::stdin();
    loop {
        println!("Hit enter to get lyrics");
        input.read_line(&mut buf).unwrap();
        match get_lyrics(&client, &spotify_auth, &genius_auth) {
            Ok(lyrics) => println!("#################################################################################\n{}\n\n", lyrics.trim()),
            Err(_) => println!("Couldn't find lyrics"),
        }
    }
}

fn setup() -> (SpotifyClient, GeniusClient) {
    let file_path = String::from("./config.json");
    let spotify_auth = Authorizer::<SpotifyClient>::from_json_file(&file_path);
    let genius_auth = Authorizer::<GeniusClient>::from_json_file(&file_path);

    (spotify_auth, genius_auth)
}

fn setup_http_client() -> reqwest::blocking::Client{
     // this cookie is super important. without it genius might return one of two different page layouts for the lyrics
     // which makes scraping much harder. 
     // The page layout changes att the cookie value 50.
    let cookie_jar = reqwest::cookie::Jar::default();
    cookie_jar.add_cookie_str(
        "_genius_ab_test_cohort=80",
        &"https://genius.com".parse::<reqwest::Url>().unwrap(),
    );

    let client = reqwest::blocking::Client::builder()
        .cookie_provider(cookie_jar.into())
        .build()
        .unwrap();

    client
}

fn get_lyrics(
    client: &reqwest::blocking::Client,
    spotify_auth: &SpotifyClient,
    genius_auth: &GeniusClient,
) -> Result<String, ()> {
    let spotify_response = spotify_auth.get_song_playing(client).unwrap();

    let mut song = spotify_response.title;
    let spotify_artist = &spotify_response.artist;

    println!(
        "The current track is: {} by {}",
        song, spotify_artist
    );

    song = remove_feat(song);

    let mut genius_response = match genius_auth.artist_and_song_search(&song,spotify_artist, client) {
        Ok(json_result) => json_result,
        Err(s) => panic!("{}", s),
    };

    let genius_hit = genius_response.find(|entry| &entry.artist == spotify_artist); 

    if genius_hit.is_none() {
        println!("found no hits for the artist");
        return Err(());
    }

    let query_url = reqwest::Url::from_str(&format!(
        "https://genius.com{}",
        genius_hit.unwrap().lyric_path
    ))
    .unwrap();

    println!("{}", &query_url);
    let response = client.get(query_url).send().unwrap();

    #[cfg(feature = "debug")]
    println!(
        "############# Response #############\n {:?}\n",
        response.headers(),
    );

    let res = response.text().unwrap();

    #[cfg(feature = "debug")]
    std::fs::write("response_final.html", &res.as_bytes()).expect("lord we fucked up");

    let document = soup::Soup::new(res.as_str());
    text::extract_lyrics(document)
}

