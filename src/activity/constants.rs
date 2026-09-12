//! Well-known string values for the stringly-typed activity fields.
//!
//! Mirrors the `*Types` / `*Hints` / `*Modes` enums of `microsoft-agents-activity`.

/// Values of [`Activity::input_hint`](super::Activity::input_hint).
pub mod input_hints {
    /// The agent is accepting input.
    pub const ACCEPTING_INPUT: &str = "acceptingInput";
    /// The agent is ignoring input.
    pub const IGNORING_INPUT: &str = "ignoringInput";
    /// The agent is expecting input.
    pub const EXPECTING_INPUT: &str = "expectingInput";
}

/// Values of [`Activity::delivery_mode`](super::Activity::delivery_mode).
pub mod delivery_modes {
    /// Default delivery.
    pub const NORMAL: &str = "normal";
    /// Notification delivery.
    pub const NOTIFICATION: &str = "notification";
    /// Replies are expected inline.
    pub const EXPECT_REPLIES: &str = "expectReplies";
    /// Ephemeral delivery.
    pub const EPHEMERAL: &str = "ephemeral";
    /// Streamed delivery.
    pub const STREAM: &str = "stream";
}

/// Values of [`Activity::code`](super::Activity::code) on `endOfConversation`.
pub mod end_of_conversation_codes {
    /// Unknown reason.
    pub const UNKNOWN: &str = "unknown";
    /// Completed successfully.
    pub const COMPLETED_SUCCESSFULLY: &str = "completedSuccessfully";
    /// The user cancelled.
    pub const USER_CANCELLED: &str = "userCancelled";
    /// The agent timed out.
    pub const BOT_TIMED_OUT: &str = "botTimedOut";
    /// The agent issued an invalid message.
    pub const BOT_ISSUED_INVALID_MESSAGE: &str = "botIssuedInvalidMessage";
    /// The channel failed.
    pub const CHANNEL_FAILED: &str = "channelFailed";
}

/// Values of [`Activity::text_format`](super::Activity::text_format).
pub mod text_format_types {
    /// Markdown (the default).
    pub const MARKDOWN: &str = "markdown";
    /// Plain text.
    pub const PLAIN: &str = "plain";
    /// XML.
    pub const XML: &str = "xml";
}

/// Values of [`Activity::attachment_layout`](super::Activity::attachment_layout).
pub mod attachment_layout_types {
    /// List layout (the default).
    pub const LIST: &str = "list";
    /// Carousel layout.
    pub const CAROUSEL: &str = "carousel";
}

/// Values of [`Activity::importance`](super::Activity::importance).
pub mod activity_importance {
    /// Low.
    pub const LOW: &str = "low";
    /// Normal.
    pub const NORMAL: &str = "normal";
    /// High.
    pub const HIGH: &str = "high";
}

/// Values of [`ChannelAccount::role`](super::ChannelAccount::role).
pub mod role_types {
    /// A user.
    pub const USER: &str = "user";
    /// An agent.
    pub const AGENT: &str = "bot";
    /// A skill.
    pub const SKILL: &str = "skill";
    /// A connector user (Copilot Studio → connector calls).
    pub const CONNECTOR_USER: &str = "connectoruser";
}

/// Values of the `type` field of [`CardAction`](super::CardAction).
pub mod action_types {
    /// Open a URL.
    pub const OPEN_URL: &str = "openUrl";
    /// Send the value back as a message from the user.
    pub const IM_BACK: &str = "imBack";
    /// Post the value back without showing it.
    pub const POST_BACK: &str = "postBack";
    /// Play audio.
    pub const PLAY_AUDIO: &str = "playAudio";
    /// Play video.
    pub const PLAY_VIDEO: &str = "playVideo";
    /// Show an image.
    pub const SHOW_IMAGE: &str = "showImage";
    /// Download a file.
    pub const DOWNLOAD_FILE: &str = "downloadFile";
    /// Sign in.
    pub const SIGNIN: &str = "signin";
    /// Place a call.
    pub const CALL: &str = "call";
    /// Send a message back.
    pub const MESSAGE_BACK: &str = "messageBack";
}

/// Values of the `type` field of [`Entity`](super::Entity).
pub mod entity_types {
    /// Activity treatment.
    pub const ACTIVITY_TREATMENT: &str = "activityTreatment";
    /// AI citation.
    pub const AI_CITATION: &str = "https://schema.org/Message";
    /// Geo coordinates.
    pub const GEO_COORDINATES: &str = "GeoCoordinates";
    /// Mention.
    pub const MENTION: &str = "mention";
    /// Place.
    pub const PLACE: &str = "Place";
    /// Thing.
    pub const THING: &str = "Thing";
    /// Product info.
    pub const PRODUCT_INFO: &str = "ProductInfo";
    /// Stream info (streamed responses).
    pub const STREAM_INFO: &str = "streaminfo";
}
