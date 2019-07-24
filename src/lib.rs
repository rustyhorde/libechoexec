#![feature(async_await)]

use hyper::{Client, Uri};
use std::io::{self, Write};
use tokio::runtime::Runtime;

// A simple type alias so as to DRY.
type FutResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn fetch_url(url: Uri) -> FutResult<()> {
    let client = Client::new();

    println!("Getting URL");
    let res = client.get(url).await?;

    println!("Response: {}", res.status());
    println!("Headers: {:#?}\n", res.headers());

    let mut body = res.into_body();

    while let Some(next) = body.next().await {
        let chunk = next?;
        io::stdout().write_all(&chunk)?;
    }

    println!("\nDone!");

    Ok(())
}

pub struct EchoRt {
    rt: Runtime,
}

impl EchoRt {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            rt: Runtime::new().unwrap(),
        })
    }

    pub fn spawn(&self) {
        self.rt.spawn(async {
            let url = "http://jira.kroger.com";
            let url = url.parse::<Uri>().unwrap();
            let res = fetch_url(url).await;
            println!("Res: {:?}", res);
        });
    }
}

#[cfg(test)]
mod test {
    use super::EchoRt;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn spawn() -> Result<(), String> {
        let echo_rt = EchoRt::new()?;
        echo_rt.spawn();

        // Sleep so the spawned future can complete
        thread::sleep(Duration::from_millis(500));
        Ok(())
    }
}
