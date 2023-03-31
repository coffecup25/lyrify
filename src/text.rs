use soup::prelude::*;
use html5ever::rcdom;


/// Extracts the lyrics from the genius page and returns them as a string
pub fn extract_lyrics(document: soup::Soup) -> Result<String, ()> {
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
/// Recursively extracts all text from a node and its children and appends it to the result vector
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

pub fn remove_feat(name: String) -> String {
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
    use crate::text::remove_feat;

    #[test]
    fn test_remove_feature_in_song_name() {
        let song_name = String::from("Skrawberries (feat. BJ The Chicago Kid)");
        let cleared_string = remove_feat(song_name);
        let correct_string = String::from("Skrawberries");

        assert_eq!(correct_string, cleared_string);

        let song_name = String::from("CASH MANIAC | CAZH MAN1AC [FEAT. NYYJERYA]");
        let cleared_string = remove_feat(song_name);
        let correct_string = String::from("CASH MANIAC | CAZH MAN1AC");

        assert_eq!(correct_string, cleared_string);

        let song_name = String::from("Beauty In The Dark (Groove With You)");
        let cleared_string = remove_feat(song_name);
        let correct_string = String::from("Beauty In The Dark (Groove With You)");

        assert_eq!(correct_string, cleared_string);
    }
}
