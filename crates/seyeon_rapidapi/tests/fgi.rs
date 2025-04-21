use seyeon_rapidapi::RapidApiClient;
use seyeon_rapidapi::fgi::FearAndGreedIndex;
use tokio::test;

#[test]
pub async fn fetch_fear_and_greed_index() {
    let client = RapidApiClient::new(
        std::env::var("RAPIDAPI_KEY")
            .expect("Fill $RAPIDAPI_KEY")
            .as_str(),
    );

    let response = client
        .call0::<FearAndGreedIndex>()
        .await
        .expect("Failed to fetch fear and greed index");

    println!("{response:?}");
}
