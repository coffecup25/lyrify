mod authorizer;
mod response;

use authorizer::Authorizer;

use response::{GeniusAuth, Response, SpotifyAuth};
use soup::prelude::*;
use std::{error::Error, io, str::FromStr};
use html5ever::rcdom;

fn main() {
    let (spotify_auth, genius_auth) = setup();

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
            Ok(lyrics) => println!("#################################################################################\n{}\n\n", lyrics.trim()),
            Err(_) => println!("Couldn't find lyrics"),
        }
    }
}

fn setup() -> (SpotifyAuth, GeniusAuth) {
    let file_path = String::from("./config.json");
    let spotify_auth = Authorizer::<SpotifyAuth>::from_json_file(&file_path);
    let genius_auth = Authorizer::<GeniusAuth>::from_json_file(&file_path);

    (spotify_auth, genius_auth)
}

fn get_lyrics(
    client: &reqwest::blocking::Client,
    spotify_auth: &SpotifyAuth,
    genius_auth: &GeniusAuth,
) -> Result<String, ()> {
    let spotify_response = spotify_auth
        .query(
            "https://api.spotify.com/v1/me/player/currently-playing",
            client,
        )
        .unwrap();

    let mut song = spotify_response.title;
    let spotify_artist = &spotify_response.artist;

    println!(
        "The currently playing track is: {} by {}",
        song, spotify_artist
    );

    song = remove_feat(&mut song);

    let mut query_url = reqwest::Url::parse_with_params(
        "https://api.genius.com/search",
        &[("q", format!("{} {}", &song, spotify_artist))],
    )
    .unwrap();

    #[cfg(feature = "debug")]
    println!("genius query url: {}", query_url);

    let mut genius_response = match genius_auth.query(&query_url.to_string(), client) {
        Ok(json_result) => json_result,
        Err(s) => panic!("{}", s),
    };

    let mut genius_hit = None;
    while let Some(entry) = genius_response.next() {
        if entry.artist == spotify_response.artist {
            genius_hit = Some(entry);
            break;
        }
    }

    if genius_hit.is_none() {
        println!("found no hits for the artist");
        return Err(());
    }

    query_url = reqwest::Url::from_str(&format!(
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
    extract_lyrics(document)
}

fn extract_lyrics(document: soup::Soup) -> Result<String, ()> {
    let root_node = match document
        .tag("div")
        .class("Lyrics__Container-sc-1ynbvzw-6")
        .find()
    {
        Some(node) => node,
        None => return Err(()),
    };

    let node = root_node.get_node();
    let mut result = vec![];
    extract_text(node, &mut result);
    Ok(result.join("\n"))
    
}

fn extract_text(node: &rcdom::Node, result: &mut Vec<String>) {
    match node.data {
        rcdom::NodeData::Text {
            ref contents, ..
        } => result.push(contents.borrow().to_string()),
        _ => (),
    }
    let children = node.children.borrow();
    for child in children.iter() {
        extract_text(child, result);
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
                    new_string.push(' ');
                }
            } else {
                new_string.push_str(word);
                new_string.push(' ');
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
