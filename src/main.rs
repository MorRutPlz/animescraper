use fantoccini::{Client, ClientBuilder, Locator};

// let's set up the sequence of steps we want the browser to take
#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    let mut client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .unwrap();

    get_link(
        &mut client,
        "https://animixplay.to/v1/yakusoku-no-neverland-2nd-season",
    )
    .await;

    for i in 2..12 {
        get_link(
            &mut client,
            &format!(
                "https://animixplay.to/v1/yakusoku-no-neverland-2nd-season/ep{}",
                i
            ),
        )
        .await;
    }

    client.close().await
}

async fn get_link(client: &mut Client, url: &str) {
    client.goto(url).await.unwrap();

    let mut frame = client
        .find(Locator::Id("iframeplayer"))
        .await
        .unwrap()
        .enter_frame()
        .await
        .unwrap();

    println!(
        "{}",
        frame
            .find(Locator::Css("source"))
            .await
            .unwrap()
            .attr("src")
            .await
            .unwrap()
            .unwrap()
    );
}
