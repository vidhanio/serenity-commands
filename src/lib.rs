#![cfg_attr(docsrs, feature(doc_auto_cfg))]
//! A library for creating/parsing [`serenity`] slash commands.
//!
//! # Examples
//!
//! ```rust
//! use serenity::all::{
//!     async_trait, Client, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
//!     EventHandler, GatewayIntents, GuildId, Interaction,
//! };
//! use serenity_commands::{Command, Commands, SubCommand};
//!
//! #[derive(Debug, Commands)]
//! enum AllCommands {
//!     /// Ping the bot.
//!     Ping,
//!
//!     /// Echo a message.
//!     Echo {
//!         /// The message to echo.
//!         message: String,
//!     },
//!
//!     /// Perform math operations.
//!     Math(MathCommand),
//! }
//!
//! impl AllCommands {
//!     fn run(self) -> String {
//!         match self {
//!             Self::Ping => "Pong!".to_string(),
//!             Self::Echo { message } => message,
//!             Self::Math(math) => math.run().to_string(),
//!         }
//!     }
//! }
//!
//! #[derive(Debug, Command)]
//! enum MathCommand {
//!     /// Add two numbers.
//!     Add(BinaryOperation),
//!
//!     /// Subtract two numbers.
//!     Subtract(BinaryOperation),
//!
//!     /// Multiply two numbers.
//!     Multiply(BinaryOperation),
//!
//!     /// Divide two numbers.
//!     Divide(BinaryOperation),
//!
//!     /// Negate a number.
//!     Negate {
//!         /// The number to negate.
//!         a: f64,
//!     },
//! }
//!
//! impl MathCommand {
//!     fn run(self) -> f64 {
//!         match self {
//!             Self::Add(BinaryOperation { a, b }) => a + b,
//!             Self::Subtract(BinaryOperation { a, b }) => a - b,
//!             Self::Multiply(BinaryOperation { a, b }) => a * b,
//!             Self::Divide(BinaryOperation { a, b }) => a / b,
//!             Self::Negate { a } => -a,
//!         }
//!     }
//! }
//!
//! #[derive(Debug, SubCommand)]
//! struct BinaryOperation {
//!     /// The first number.
//!     a: f64,
//!
//!     /// The second number.
//!     b: f64,
//! }
//!
//! struct Handler {
//!     guild_id: GuildId,
//! }
//!
//! #[async_trait]
//! impl EventHandler for Handler {
//!     async fn ready(&self, ctx: Context, _: serenity::all::Ready) {
//!         self.guild_id
//!             .set_commands(&ctx, AllCommands::create_commands())
//!             .await
//!             .unwrap();
//!     }
//!
//!     async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
//!         if let Interaction::Command(command) = interaction {
//!             let command_data = AllCommands::from_command_data(&command.data).unwrap();
//!             command
//!                 .create_response(
//!                     ctx,
//!                     CreateInteractionResponse::Message(
//!                         CreateInteractionResponseMessage::new().content(command_data.run()),
//!                     ),
//!                 )
//!                 .await
//!                 .unwrap();
//!         }
//!     }
//! }
//! ```

use serenity::all::{
    AttachmentId, ChannelId, CommandData, CommandDataOption, CommandDataOptionValue,
    CommandOptionType, CreateCommand, CreateCommandOption, GenericId, RoleId, UserId,
};
/// Derives [`BasicOption`].
///
/// `option_type` can be `"string"`, `"integer"`, or `"number"`.
///
/// # Examples
///
/// ```rust
/// use serenity_commands::BasicOption;
///
/// #[derive(Debug, BasicOption)]
/// #[choice(option_type = "integer")]
/// enum Medal {
///     #[choice(name = "Gold", value = 1)]
///     Gold,
///
///     #[choice(name = "Silver", value = 2)]
///     Silver,
///
///     #[choice(name = "Bronze", value = 3)]
///     Bronze,
/// }
/// ```
pub use serenity_commands_macros::BasicOption;
/// Derives [`Command`].
///
/// # Examples
///
/// ## Struct
///
/// Each field must implement [`BasicOption`].
///
/// ```rust
/// use serenity_commands::Command;
///
/// #[derive(Command)]
/// struct Add {
///     /// First number.
///     a: f64,
///
///     /// Second number.
///     b: f64,
/// }
/// ```
///
/// ## Enum
///
/// Each field of named variants must implement [`BasicOption`].
///
/// The inner type of newtype variants must implement [`SubCommandGroup`] (or,
/// by extension, [`SubCommand`], as [`SubCommand`] is a sub-trait of
/// [`SubCommandGroup`]).
///
/// ```rust
/// use serenity_commands::{Command, SubCommandGroup};
///
/// #[derive(SubCommandGroup)]
/// enum ModUtilities {
///     // ...
/// }
///
/// #[derive(Command)]
/// enum Utilities {
///     /// Ping the bot.
///     Ping,
///
///     /// Add two numbers.
///     Add {
///         /// First number.
///         a: f64,
///
///         /// Second number.
///         b: f64,
///     },
///
///     /// Moderation utilities.
///     Mod(ModUtilities),
/// }
pub use serenity_commands_macros::Command;
/// Derives [`Commands`].
///
/// # Examples
///
/// Each field of named variants must implement [`Command`].
///
/// The inner type of newtype variants must implement [`Command`].
///
/// ```rust
/// use serenity_commands::{Command, Commands};
///
/// #[derive(Command)]
/// enum MathCommand {
///     // ...
/// }
///
/// #[derive(Commands)]
/// enum AllCommands {
///     /// Ping the bot.
///     Ping,
///
///     /// Echo a message.
///     Echo {
///         /// The message to echo.
///         message: String,
///     },
///
///     /// Do math operations.
///     Math(MathCommand),
/// }
pub use serenity_commands_macros::Commands;
/// Derives [`SubCommand`].
///
/// Each field must implement [`BasicOption`].
///
/// # Examples
///
/// ```rust
/// use serenity_commands::SubCommand;
///
/// #[derive(SubCommand)]
/// struct Add {
///     /// First number.
///     a: f64,
///
///     /// Second number.
///     b: f64,
/// }
/// ```
pub use serenity_commands_macros::SubCommand;
/// Derives [`SubCommandGroup`].
///
/// Each field of named variants must implement [`BasicOption`].
///
/// The inner type of newtype variants must implement [`SubCommand`].
///
/// # Examples
///
/// ```rust
/// use serenity_commands::{SubCommand, SubCommandGroup};
///
/// #[derive(SubCommand)]
/// struct AddSubCommand {
///     /// First number.
///     a: f64,
///
///     /// Second number.
///     b: f64,
/// }
///
/// #[derive(SubCommandGroup)]
/// enum Math {
///     /// Add two numbers.
///     Add(AddSubCommand),
///
///     /// Negate a number.
///     Negate {
///         /// The number to negate.
///         a: f64,
///     },
/// }
pub use serenity_commands_macros::SubCommandGroup;
use thiserror::Error;

/// A type alias for [`std::result::Result`]s which use [`Error`] as the error
/// type.
///
/// [`Error`]: enum@Error
pub type Result<T> = std::result::Result<T, Error>;

/// An error which can occur when extracting data from a command interaction.
#[derive(Debug, Error)]
pub enum Error {
    /// An unknown command was provided.
    #[error("unknown command: {0}")]
    UnknownCommand(String),

    /// An incorrect command option type was provided.
    #[error("incorrect command option type: got {got:?}, expected {expected:?}")]
    IncorrectCommandOptionType {
        /// The type of command option that was provided.
        got: CommandOptionType,

        /// The type of command option that was expected.
        expected: CommandOptionType,
    },

    /// An incorrect number of command options were provided.
    #[error("incorrect command option count: got {got}, expected {expected}")]
    IncorrectCommandOptionCount {
        /// The number of command options that were provided.
        got: usize,

        /// The number of command options that were expected.
        expected: usize,
    },

    /// An unknown command option was provided.
    #[error("unknown command option: {0}")]
    UnknownCommandOption(String),

    /// A required command option was not provided.
    #[error("required command option not provided")]
    MissingRequiredCommandOption,

    /// An unknown choice was provided.
    #[error("unknown choice: {0}")]
    UnknownChoice(String),

    /// An error occurred within a custom implementation.
    #[error(transparent)]
    Custom(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A utility for creating commands and extracting their data from application
/// commands.
pub trait Commands: Sized {
    /// List of top-level commands.
    fn create_commands() -> Vec<CreateCommand>;

    /// Extract data from [`CommandData`].
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_command_data(data: &CommandData) -> Result<Self>;
}

/// A top-level command for use with [`Commands`].
pub trait Command: Sized {
    /// Create the command.
    fn create_command(name: impl Into<String>, description: impl Into<String>) -> CreateCommand;

    /// Extract data from a list of [`CommandDataOption`]s.
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_options(options: &[CommandDataOption]) -> Result<Self>;
}

/// A sub-command group which can be nested inside of a [`Command`] and contains
/// [`SubCommand`]s.
pub trait SubCommandGroup: Sized {
    /// Create the command option.
    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption;

    /// Extract data from a [`CommandDataOptionValue`].
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_value(value: &CommandDataOptionValue) -> Result<Self>;
}

/// A sub-command which can be nested inside of a [`Command`] or
/// [`SubCommandGroup`].
///
/// This is a sub-trait of [`SubCommandGroup`], as a [`SubCommand`] can be used
/// anywhere a [`SubCommandGroup`] can.
pub trait SubCommand: SubCommandGroup {
    /// Create the command option.
    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption {
        <Self as SubCommandGroup>::create_option(name, description)
    }

    /// Extract data from a [`CommandDataOption`].
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_value(value: &CommandDataOptionValue) -> Result<Self> {
        <Self as SubCommandGroup>::from_value(value)
    }
}

/// A basic option which can be nested inside of [`Command`]s or
/// [`SubCommand`]s.
///
/// This trait is implemented already for most primitive types.
pub trait BasicOption: Sized {
    /// Create the command option.
    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption;

    /// Extract data from a [`CommandDataOptionValue`].
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_value(value: Option<&CommandDataOptionValue>) -> Result<Self>;
}

macro_rules! impl_command_option {
    ($($Variant:ident($($Ty:ty),* $(,)?)),* $(,)?) => {
        $($(
            impl BasicOption for $Ty {
                fn create_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption {
                    CreateCommandOption::new(CommandOptionType::$Variant, name, description)
                        .required(true)
                }

                fn from_value(value: Option<&CommandDataOptionValue>) -> Result<Self> {
                    let value = value.ok_or(Error::MissingRequiredCommandOption)?;

                    match value {
                        CommandDataOptionValue::$Variant(v) => Ok(v.clone() as _),
                        _ => Err(Error::IncorrectCommandOptionType {
                            got: value.kind(),
                            expected: CommandOptionType::$Variant,
                        }),
                    }
                }
            }
        )*)*
    };
}

impl_command_option! {
    Boolean(bool),
    String(String),
    Attachment(AttachmentId),
    Channel(ChannelId),
    Mentionable(GenericId),
    Role(RoleId),
    User(UserId),
}

macro_rules! impl_number_command_option {
    ($($Ty:ty),* $(,)?) => {
        $(
            impl BasicOption for $Ty {
                fn create_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption {
                    CreateCommandOption::new(CommandOptionType::Number, name, description)
                        .required(true)
                }

                fn from_value(value: Option<&CommandDataOptionValue>) -> Result<Self> {
                    let value = value.ok_or(Error::MissingRequiredCommandOption)?;

                    match value {
                        CommandDataOptionValue::Number(v) => Ok(*v as _),
                        _ => Err(Error::IncorrectCommandOptionType {
                            got: value.kind(),
                            expected: CommandOptionType::Number,
                        }),
                    }
                }
            }

        )*
    };
}

impl_number_command_option!(f32, f64);

macro_rules! impl_integer_command_option {
    ($($Ty:ty),* $(,)?) => {
        $(
            impl BasicOption for $Ty {
                fn create_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption {
                    CreateCommandOption::new(CommandOptionType::Integer, name, description)
                        .required(true)
                }

                fn from_value(value: Option<&CommandDataOptionValue>) -> Result<Self> {
                    let value = value.ok_or(Error::MissingRequiredCommandOption)?;

                    match value {
                        CommandDataOptionValue::Integer(v) => Ok(*v as _),
                        _ => Err(Error::IncorrectCommandOptionType {
                            got: value.kind(),
                            expected: CommandOptionType::Integer,
                        }),
                    }
                }
            }
        )*
    };
}

impl_integer_command_option!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl<T: BasicOption> BasicOption for Option<T> {
    /// Delegates to `T`'s [`BasicOption::create_option`] implementation, but
    /// sets [`CreateCommandOption::required`] to `false` afterwards.
    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption {
        T::create_option(name, description).required(false)
    }

    /// Only delegates to `T`'s [`BasicOption::from_value`] implementation if
    /// `value` is [`Some`].
    fn from_value(value: Option<&CommandDataOptionValue>) -> Result<Self> {
        value.map(|option| T::from_value(Some(option))).transpose()
    }
}
