# Serenity Commands

A set of traits/derive macros for intuitively creating and parsing application commands from [Serenity](https://github/serenity-rs/serenity).

## Usage

```rust
use serenity::all::{async_trait, Context, EventHandler, Interaction, Ready};
use serenity_commands::{Command, CommandData, CommandOption};

#[derive(Debug, CommandData)]
enum AllCommands {
    /// Ping the bot.
    Ping,

    /// Echo a message.
    Echo {
        /// The message to echo.
        message: String,
    },

    /// Math operations.
    MathCommand(MathCommand),
}

#[derive(Debug, Command)]
enum MathCommand {
    /// Add two numbers.
    Add(BinaryOperation),

    /// Subtract two numbers.
    Subtract(BinaryOperation),

    /// Negate a number.
    Negate {
        /// The number to negate.
        a: i32,
    },

    /// Raise a number to a power.
    Power {
        /// The number to raise.
        a: i32,

        /// The power to raise to.
        b: i32,
    },
}

#[derive(Debug, CommandOption)]
struct BinaryOperation {
    /// The first number.
    a: i32,

    /// The second number.
    b: i32,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        serenity::all::Command::set_global_commands(&ctx, AllCommands::to_command_data()).await.unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let data = AllCommands::from_command_data(&command.data).unwrap();
            println!("{data:#?}");
        }
    }
}
```
