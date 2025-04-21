pub trait Method0 {
    const PATH: &'static str;
    type Response: serde::de::DeserializeOwned;
}
