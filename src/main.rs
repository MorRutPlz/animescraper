mod model;

use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use model::{Animix, GogoStream};
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;
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

    let list_a = list
        .iter()
        .filter(|x| x.e == "1")
        .map(|x| (x, format!("https://animixplay.to/v1/{}", x.id)))
        .collect::<Vec<_>>();

    let pb = ProgressBar::new(list_a.len() as u64);

    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-"),
    );

    futures::stream::iter(
        list_a
            .into_iter()
            .map(|(anime, url)| (anime, url, pb.clone(), success.clone(), errors.clone()))
            .map(|(anime, i, pb, success, errors)| async move {
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

                pb.inc(1);
                pb.set_message(&anime.title);
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

        scrap_further(link.clone()).await;

        episodes.push((index, link));
    }

    episodes.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(episodes.into_iter().map(|(_, b)| b).collect())
}

async fn scrap_further(link: String) {
    let mut sources = HashMap::new();

    let mut split = link.split("?id=");
    split.next();

    let video_id = match split.next() {
        Some(n) => match n.split("&").next() {
            Some(n) => n,
            None => return,
        },
        None => return,
    };

    // VIDSTREAMING
    // https://gogo-play.net/ajax.php?id=MTUwNTEw

    match reqwest::get(&format!("https://gogo-play.net/ajax.php?id={}", video_id)).await {
        Ok(n) => match n.json::<GogoStream>().await {
            Ok(n) => {
                let mut gogos = Vec::new();

                match n.source {
                    Some(n) => gogos.push(n),
                    None => {}
                }

                match n.source_bk {
                    Some(n) => gogos.push(n),
                    None => {}
                }

                if gogos.len() > 0 {
                    sources.insert("ANIMIX-GOGO", gogos);
                }
            }
            Err(e) => {}
        },
        Err(e) => {}
    }

    // MULTI QUALITY
    // https://gogo-play.net/loadserver.php?id=MTUwNTEw

    match reqwest::get(&format!(
        "https://gogo-play.net/loadserver.php?id={}",
        video_id
    ))
    .await
    {
        Ok(n) => match n.json::<GogoStream>().await {
            Ok(n) => {
                let mut gogos = Vec::new();

                match n.source {
                    Some(n) => gogos.push(n),
                    None => {}
                }

                match n.source_bk {
                    Some(n) => gogos.push(n),
                    None => {}
                }

                if gogos.len() > 0 {
                    sources.insert("ANIMIX-GOGO", gogos);
                }
            }
            Err(e) => {}
        },
        Err(e) => {}
    }
}
