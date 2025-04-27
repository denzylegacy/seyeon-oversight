use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SocialStats {
    pub reddit: RedditStats,
    pub twitter: TwitterStats,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RedditStats {
    pub avg_active_users: f64,
    pub subscribers: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TwitterStats {
    pub followers_count: i32,
    pub status_count: i32,
}