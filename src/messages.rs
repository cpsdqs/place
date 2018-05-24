use base64;
use canvas::Region;
use serde_json;
use ws;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientRequest {
    #[serde(rename = "set-pixel")]
    SetPixel { x: u32, y: u32, r: u8, g: u8, b: u8 },

    #[serde(rename = "chat-message")]
    ChatMessage { x: f32, y: f32, text: String },

    #[serde(rename = "admin-auth")]
    AdminAuth(String),

    #[serde(rename = "admin-console")]
    AdminConsole(String),

    #[serde(rename = "admin-broadcast")]
    AdminBroadcast { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RGBARegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub data: String,
}

impl From<Region> for RGBARegion {
    fn from(region: Region) -> RGBARegion {
        let mut rgba_data = Vec::with_capacity(region.data.len() * 4 / 3);

        for i in 0..(region.data.len() / 3) {
            rgba_data.push(region.data[i * 3 + 0]);
            rgba_data.push(region.data[i * 3 + 1]);
            rgba_data.push(region.data[i * 3 + 2]);
            rgba_data.push(255);
        }

        RGBARegion {
            x: region.x,
            y: region.y,
            w: region.w,
            h: region.h,
            data: base64::encode(&rgba_data),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    #[serde(rename = "full-update")]
    FullUpdate { w: u32, h: u32, data: String },

    #[serde(rename = "regions")]
    Regions(Vec<RGBARegion>),

    #[serde(rename = "error")]
    Error { code: String, message: String },

    #[serde(rename = "chat-message")]
    ChatMessage {
        x: f32,
        y: f32,
        text: String,
        id_hue: Option<f32>,
        is_admin: bool,
    },
}

impl Into<ws::Message> for ClientMessage {
    fn into(self) -> ws::Message {
        ws::Message::Text(serde_json::to_string(&self).unwrap())
    }
}
