#![allow(missing_docs, dead_code)]

use serenity::{
    all::{GatewayIntents, GuildId, Interaction},
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::{Context, EventHandler},
    Client,
};
use serenity_commands::{Command, CommandData, CommandOption};

#[derive(Debug, CommandData)]
enum Commands {
    /// Ping the bot.
    Ping,

    /// Echo a message.
    Echo {
        /// The message to echo.
        message: String,
    },

    /// Perform math operations.
    Math(MathCommand),

    /// one or two numbers.
    OneOrTwo(OneOrTwo),

    /// Miscaellaneaous commands.
    Misc(MiscCommands),
}

#[derive(Debug, Command)]
enum MathCommand {
    /// Add two numbers.
    Add {
        /// The first number.
        first: f64,

        /// The second number.
        second: f64,
    },

    /// Subtract two numbers.
    Subtract(SubtractCommandOption),
}

#[derive(Debug, CommandOption)]
struct SubtractCommandOption {
    /// The first number.
    first: f64,

    /// The second number.
    second: f64,
}

#[derive(Debug, Command)]
enum MiscCommands {
    /// Get the current time.
    Time,

    /// one or two numbers... inside misc!
    OneOrTwo(OneOrTwo),
    // /// deeper misc commands
    // Deeper(DeeperMiscCommands), DOES NOT COMPILE! nesting 3 levels deep is not supported by the
    // discord API, and thus this crate prevents it.
}

#[derive(Debug, Command)]
enum DeeperMiscCommands {
    /// how??
    How,
}

// usable at the top level or as a subcommand!
#[derive(Debug, Command, CommandOption)]
struct OneOrTwo {
    /// The first number.
    first: f64,

    /// The second number, optional.
    second: Option<f64>,
}

struct Handler {
    guild_id: GuildId,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: serenity::model::gateway::Ready) {
        self.guild_id
            .set_commands(&ctx, Commands::to_command_data())
            .await
            .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let command_data = Commands::from_command_data(&command.data).unwrap();
            command
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("```rs\n{command_data:?}```")),
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
