mod model;

use futures::StreamExt;
use model::Animix;
use scraper::{Html, Selector};
use serde_json::Value;
use std::fs;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let list =
        serde_json::from_str::<Vec<Animix>>(&fs::read_to_string("all.json").unwrap()).unwrap();

    let success = Arc::new(Mutex::new(
        OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open("success.txt")
            .await
            .unwrap(),
    ));

    let errors = Arc::new(Mutex::new(
        OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open("errors.txt")
            .await
            .unwrap(),
    ));

    futures::stream::iter(
        list.iter()
            .filter(|x| x.e == "1")
            .map(|x| (x, format!("https://animixplay.to/v1/{}", x.id)))
            .map(|(anime, url)| (anime, url, success.clone(), errors.clone()))
            .map(|(anime, i, success, errors)| async move {
                println!("Attempting `{}`", anime.title);

                match reqwest::get(&i).await {
                    Ok(resp) => match resp.text().await {
                        Ok(n) => match get_episodes(n).await {
                            Ok(n) => {
                                let entry = format!(
                                    "Title: {}; URL: {}; Episodes ({}):\n{:#?}\n\n",
                                    anime.title,
                                    i,
                                    n.len(),
                                    n
                                );

                                success
                                    .lock()
                                    .await
                                    .write_all(entry.as_bytes())
                                    .await
                                    .unwrap();
                            }
                            Err(e) => {
                                errors
                                    .lock()
                                    .await
                                    .write_all(format!("URL: {}; Message: {}\n", i, e).as_bytes())
                                    .await
                                    .unwrap();
                            }
                        },
                        Err(e) => {
                            panic!("failed to read text from response with url `{}`: {}", i, e)
                        }
                    },
                    Err(e) => {
                        panic!("failed to send GET request with url `{}`: {}", i, e)
                    }
                }
            }),
    )
    .buffer_unordered(8)
    .collect::<Vec<()>>()
    .await;
}

async fn get_episodes(response: String) -> Result<Vec<String>, String> {
    let html = Html::parse_document(&response);
    let selector = Selector::parse("#epslistplace").unwrap();
    let results = html.select(&selector).collect::<Vec<_>>();

    let value = match serde_json::from_str::<Value>(&results[0].inner_html()) {
        Ok(n) => n,
        Err(e) => return Err(format!("couldn't parse json: {}", e)),
    };

    let map = if results.len() > 0 {
        match value.as_object() {
            Some(n) => n,
            None => return Err(format!("not a JSON object")),
        }
    } else {
        return Err(format!("#epslistplace not found"));
    };

    let mut episodes = Vec::new();

    for (k, v) in map.iter().filter(|(k, _)| !k.contains("eptotal")) {
        let index = match k.parse::<usize>() {
            Ok(n) => n,
            Err(_) => return Err(format!("invalid index")),
        };

        let link = match v.as_str() {
            Some(n) => n.to_owned(),
            None => return Err(format!("link not a string")),
        };

        episodes.push((index, link));
    }

    episodes.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(episodes.into_iter().map(|(_, b)| b).collect())
}
