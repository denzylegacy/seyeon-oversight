use seyeon_cryptocompare::histoday::{Histoday, HistodayParams};
use tokio::test;

#[test]
pub async fn fetch_histoday() {
    let client = seyeon_cryptocompare::CryptocompareClient::new(
        std::env::var("CRYPTOCOMPARE_API_KEY")
            .expect("Fill $CRYPTOCOMPARE_API_KEY")
            .as_str(),
    );

    let response = client
        .call::<Histoday>(
            HistodayParams::builder()
                .source_sym("BTC")
                .target_sym("USD")
                .limit(2000)
                .build(),
        )
        .await
        .expect("Failed to fetch histoday");

    println!("{response:?}");
}
