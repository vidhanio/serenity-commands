#![allow(missing_docs, dead_code)]

use std::{
    fmt::{self, Display},
    ops::Add,
};

use serenity::all::{
    async_trait, Client, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
    EventHandler, GatewayIntents, GuildId, Interaction,
};
use serenity_commands::{BasicOption, Command, Commands};

#[derive(Debug, Commands)]
#[allow(clippy::enum_variant_names)]
enum AllCommands {
    /// Add two integers together.
    AddInts(AddCommand<u64>),

    /// Add two floats together.
    AddFloats(AddCommand<f64>),

    /// Add two vectors together.
    AddVec2s(AddCommand<Vec2>),
}

enum AllCommandsAutocomplete {
    AddInts(AddCommand<u64>),
    AddFloats(AddCommand<f64>),
    AddVec2s(AddCommand<Vec2>),
}

impl AllCommands {
    fn run(self) -> String {
        match self {
            Self::AddInts(add) => add.run().to_string(),
            Self::AddFloats(add) => add.run().to_string(),
            Self::AddVec2s(add) => add.run().to_string(),
        }
    }
}

#[derive(Debug, Command)]
struct AddCommand<T: BasicOption + Add> {
    /// The first thing.
    a: T,

    /// The second thing.
    b: T,
}

impl<T: BasicOption + Add> AddCommand<T> {
    fn run(self) -> T::Output {
        self.a + self.b
    }
}

#[derive(Debug)]
struct Vec2 {
    x: f64,
    y: f64,
}

impl BasicOption for Vec2 {
    type Partial = String;

    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> serenity::all::CreateCommandOption {
        String::create_option(name, description)
    }

    fn from_value(
        value: Option<&serenity::all::CommandDataOptionValue>,
    ) -> serenity_commands::Result<Self> {
        let value = String::from_value(value)?;

        let (x, y) = value
            .split_once(',')
            .ok_or_else(|| serenity_commands::Error::Custom("expected comma".into()))?;

        Ok(Self {
            x: x.parse()
                .map_err(|_| serenity_commands::Error::Custom("expected float".into()))?,
            y: y.parse()
                .map_err(|_| serenity_commands::Error::Custom("expected float".into()))?,
        })
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

struct Handler {
    guild_id: GuildId,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: serenity::all::Ready) {
        self.guild_id
            .set_commands(&ctx, AllCommands::create_commands())
            .await
            .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let command_data = AllCommands::from_command_data(&command.data).unwrap();
            command
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(command_data.run()),
                    ),
                )
                .await
                .unwrap();
        }
    }
}

#[tokio::main]
pub async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("expected `DISCORD_TOKEN` to be set");
    let guild_id = std::env::var("DISCORD_GUILD_ID")
        .expect("expected `DISCORD_GUILD_ID` to be set")
        .parse()
        .expect("expected `DISCORD_GUILD_ID` to be a valid guild ID");

    let mut client = Client::builder(token, GatewayIntents::non_privileged())
        .event_handler(Handler { guild_id })
        .await
        .expect("client should be created successfully");

    client
        .start()
        .await
        .expect("client should start successfully");
}
