use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};

pub const PROMPT_COLOR: u32 = 0x5865F2;
pub const PENDING_COLOR: u32 = 0xFFA726;
pub const SUCCESS_COLOR: u32 = 0x43A047;
pub const FAILURE_COLOR: u32 = 0xE53935;
pub const INFO_COLOR: u32 = 0x673AB7;

pub fn prompt(prompt: &str) -> EmbedBuilder {
    EmbedBuilder::new()
        .title("Prompt")
        .color(PROMPT_COLOR)
        .field(EmbedFieldBuilder::new("Content", prompt))
}

pub fn pending(title: &str, description: &str) -> EmbedBuilder {
    EmbedBuilder::new()
        .title(title)
        .color(PENDING_COLOR)
        .description(description)
}

pub fn success() -> EmbedBuilder {
    EmbedBuilder::new().title("Success").color(SUCCESS_COLOR)
}

pub fn failure(error: &str) -> EmbedBuilder {
    EmbedBuilder::new()
        .title("Failure")
        .color(FAILURE_COLOR)
        .description(format!("```\n{}\n```", error))
}

pub fn info() -> EmbedBuilder {
    EmbedBuilder::new().color(INFO_COLOR)
}
