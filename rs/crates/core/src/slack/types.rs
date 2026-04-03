use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SlackPayload {
    pub blocks: Vec<SlackBlock>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlackBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SlackText>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlackText {
    #[serde(rename = "type")]
    pub text_type: String,
    pub text: String,
}

impl SlackBlock {
    pub fn header(text: &str) -> Self {
        Self {
            block_type: "header".to_string(),
            text: Some(SlackText {
                text_type: "plain_text".to_string(),
                text: text.to_string(),
            }),
        }
    }

    pub fn section_markdown(text: &str) -> Self {
        Self {
            block_type: "section".to_string(),
            text: Some(SlackText {
                text_type: "mrkdwn".to_string(),
                text: text.to_string(),
            }),
        }
    }

    pub fn divider() -> Self {
        Self {
            block_type: "divider".to_string(),
            text: None,
        }
    }
}

