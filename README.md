# Serenity Commands

A library for creating/parsing [Serenity](https://github.com/serenity-rs/serenity) slash commands.

## Usage

See the [`examples`](examples) directory for more examples.

```rust
use serenity::all::{
    async_trait, Client, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
    EventHandler, GatewayIntents, GuildId, Interaction,
};
use serenity_commands::{Command, Commands, SubCommand};

#[derive(Debug, Commands)]
enum AllCommands {
    /// Ping the bot.
    Ping,

    /// Echo a message.
    Echo {
        /// The message to echo.
        message: String,
    },

    /// Perform math operations.
    Math(MathCommand),
}

impl AllCommands {
    fn run(self) -> String {
        match self {
            Self::Ping => "Pong!".to_string(),
            Self::Echo { message } => message,
            Self::Math(math) => math.run().to_string(),
        }
    }
}

#[derive(Debug, Command)]
enum MathCommand {
    /// Add two numbers.
    Add(BinaryOperation),

    /// Subtract two numbers.
    Subtract(BinaryOperation),

    /// Multiply two numbers.
    Multiply(BinaryOperation),

    /// Divide two numbers.
    Divide(BinaryOperation),

    /// Negate a number.
    Negate {
        /// The number to negate.
        a: f64,
    },
}

impl MathCommand {
    fn run(self) -> f64 {
        match self {
            Self::Add(BinaryOperation { a, b }) => a + b,
            Self::Subtract(BinaryOperation { a, b }) => a - b,
            Self::Multiply(BinaryOperation { a, b }) => a * b,
            Self::Divide(BinaryOperation { a, b }) => a / b,
            Self::Negate { a } => -a,
        }
    }
}

#[derive(Debug, SubCommand)]
struct BinaryOperation {
    /// The first number.
    a: f64,

    /// The second number.
    b: f64,
}

struct Handler {
    guild_id: GuildId,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: serenity::model::gateway::Ready) {
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
```
