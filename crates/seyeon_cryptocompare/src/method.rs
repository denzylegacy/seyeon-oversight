use serde::Serialize;

pub trait Method {
    const PATH: &'static str;

    type Response: serde::de::DeserializeOwned;
    type Params: Serialize;
}
