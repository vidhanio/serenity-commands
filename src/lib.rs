#![cfg_attr(docsrs, feature(doc_auto_cfg))]
//! A library for intuitively creating commands for use with the [`serenity`]
//! Discord library.

use serenity::all::{
    AttachmentId, ChannelId, CommandDataOption, CommandDataOptionValue, CommandOptionType,
    CreateCommand, CreateCommandOption, GenericId, RoleId, UserId,
};
/// Derive the [`Command`] trait.
///
/// This creates a top-level command for use with [`CommandData`]. The command
/// may contain regular options, sub-commands, and sub-command groups.
///
/// Documentation comments (`///`) will be used as the commands'/options'
/// descriptions, and are required whenever they are expected.
///
/// # Examples
///
/// ## `struct`s with named fields
///
/// A command with the specified options. Note that none of the fields can be
/// sub-commands or sub-command groups, and the macro will emit an error during
/// compilation if this is the case.
///
/// ```rust
/// use serenity_commands::Command;
///
/// #[derive(Command)]
/// struct Add {
///     /// The first number.
///     first: f64,
///
///     /// The second number.
///     second: f64,
/// }
/// ```
///
/// ## Newtype `struct`s
///
/// Delegates the implementation to the inner type, which must implement
/// [`Command`].
///
/// ```rust
/// use serenity_commands::Command;
///
/// # #[derive(Command)]
/// # struct InnerCommand;
/// #
/// #[derive(Command)]
/// struct CommandWrapper(InnerCommand);
/// ```
///
/// ## Unit `struct`s
///
/// A command with no options.
///
/// ```rust
/// use serenity_commands::Command;
///
/// #[derive(Command)]
/// struct Ping;
/// ```
///
/// ## `enum`s
///
/// A command with sub-commands. Note that the macro will emit an error during
/// compilation if any of the variants are not sub-commands or sub-command
/// groups.
///
/// The behaviour for each variant type is analagous to that of the
/// corresponding `struct` type:
///
/// - A variant with named fields is a sub-command with the specified options.
/// - A newtype variant is a sub-command/sub-command group which delegates the
///   implementation to the inner type, which must implement [`CommandOption`].
/// - A unit variant is a sub-command with no options.
///
/// ```rust
/// use serenity_commands::Command;
///
/// # #[derive(serenity_commands::CommandOption)]
/// # struct MathCommand;
/// #
/// #[derive(Command)]
/// enum MyCommands {
///     /// Ping the bot.
///     Ping,
///
///     /// Echo a message.
///     Echo {
///         /// The message to echo.
///         message: String,
///     },
///
///     /// Perform math operations.
///     Math(MathCommand),
/// }
/// ```
pub use serenity_commands_macros::Command;
/// Derive the [`CommandData`] trait.
///
/// This creates a top-level utility structure which can list all of its
/// commands (for use with [`GuildId::set_commands`], etc.) and extract data
/// from [`CommandInteraction`]s.
///
/// This macro only supports `enum`s, as it is intended to select from one of
/// many commands.
///
/// Documentation comments (`///`) will be used as the commands'/options'
/// descriptions, and are required whenever they are expected.
///
/// # Examples
///
/// - A variant with named fields is a command with the specified options.
/// - A newtype variant is a command which delegates the implementation to the
///   inner type, which must implement [`Command`].
/// - A unit variant is a command with no options.
///
/// ```rust
/// use serenity_commands::CommandData;
///
/// # #[derive(serenity_commands::Command)]
/// # struct MathCommand;
/// #
/// #[derive(CommandData)]
/// enum Commands {
///     /// Ping the bot.
///     Ping,
///
///     /// Echo a message.
///     Echo {
///         /// The message to echo.
///         message: String,
///     },
///
///     /// Perform math operations.
///     Math(MathCommand),
/// }
/// ```
///
/// [`GuildId::set_commands`]: serenity::all::GuildId::set_commands
/// [`CommandInteraction`]: serenity::all::CommandInteraction
pub use serenity_commands_macros::CommandData;
/// Derive the [`CommandOption`] trait.
///
/// This creates a sub-command or sub-command group which can be nested
/// within other [`CommandOption`]s or [`Command`]s.
///
/// Documentation comments (`///`) will be used as the options' descriptions,
/// and are required whenever they are expected.
///
/// # Examples
///
/// ## `struct`s with named fields
///
/// A sub-command with the specified options. Note that none of the fields
/// can be sub-commands or sub-command groups, and the macro will emit an
/// error during compilation if this is the case.
///
/// Sets [`CommandOption::TYPE`] to [`SubCommand`].
///
/// ```rust
/// use serenity_commands::CommandOption;
///
/// #[derive(CommandOption)]
/// struct AddNumbers {
///     /// The first number.
///     first: f64,
///
///     /// The second number, optional.
///     second: Option<f64>,
/// }
/// ```
///
/// ## Newtype `struct`s
///
/// Delegates the implementation to the inner type, which must implement
/// [`CommandOption`].
///
/// Sets [`CommandOption::TYPE`] to the inner type's [`CommandOption::TYPE`].
///
/// ```rust
/// use serenity_commands::CommandOption;
///
/// #[derive(CommandOption)]
/// struct FloatWrapper(f64);
/// ```
///
/// ## Unit `struct`s
///
/// A sub-command with no options.
///
/// Sets [`CommandOption::TYPE`] to [`SubCommand`].
///
/// ```rust
/// use serenity_commands::CommandOption;
///
/// #[derive(CommandOption)]
/// struct Ping;
/// ```
///
/// ## `enum`s
///
/// Sets [`CommandOption::TYPE`] to [`SubCommandGroup`].
///
/// A sub-command group. Note that the macro will emit an
/// error during compilation if any of the variants are not sub-commands or
/// sub-command groups.
///
/// The behaviour for each variant type is analagous to that of the
/// corresponding `struct` type:
///
/// - A variant with named fields is a sub-command with the specified options.
/// - A newtype variant is a sub-command/sub-command group which
///  delegates the implementation to the inner type, which must implement
/// [`CommandOption`].
/// - A unit variant is a sub-command with no options.
///
/// ```rust
/// use serenity_commands::CommandOption;
///
/// # #[derive(CommandOption)]
/// # struct MathSubCommand;
/// #
/// #[derive(CommandOption)]
/// enum MyCommands {
///     /// Ping the bot.
///     Ping,
///
///     /// Echo a message.
///     Echo {
///         /// The message to echo.
///         message: String,
///     },
///
///     /// Perform math operations.
///     Math(MathSubCommand),
/// }
/// ```
///
/// [`SubCommand`]: serenity::all::CommandOptionType::SubCommand
/// [`SubCommandGroup`]: serenity::all::CommandOptionType::SubCommandGroup
pub use serenity_commands_macros::CommandOption;
use thiserror::Error;

/// A type alias for [`Result`]s which use [`Error`] as the error type.
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

    /// An error occurred within a custom implementation.
    #[error(transparent)]
    Custom(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A top-level utility structure which can list all of its commands (for use
/// with [`GuildId::set_commands`], etc.) and extract data from
/// [`CommandInteraction`]s.
///
/// [`GuildId::set_commands`]: serenity::all::GuildId::set_commands
/// [`CommandInteraction`]: serenity::all::CommandInteraction
pub trait CommandData: Sized {
    /// List all of the commands that this type represents.
    fn to_command_data() -> Vec<CreateCommand>;

    /// Extract data from a [`serenity::all::CommandData`].
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_command_data(data: &serenity::all::CommandData) -> Result<Self>;
}

/// A top-level command for use with [`CommandData`]. The command may contain
/// regular options, sub-commands, and sub-command groups.
pub trait Command: Sized {
    /// Create the command.
    fn to_command(name: impl Into<String>, description: impl Into<String>) -> CreateCommand;

    /// Extract this command's data from an option list.
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_command(options: &[CommandDataOption]) -> Result<Self>;
}

/// A sub-command or sub-command group which can be nested within other
/// [`CommandOption`]s or [`Command`]s.
pub trait CommandOption: Sized {
    /// The type of this command option.
    const TYPE: CommandOptionType;

    /// Create the command option.
    fn to_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption;

    /// Extract this command option's data from an option list.
    ///
    /// # Errors
    ///
    /// Returns an error if the implementation fails.
    fn from_option(option: Option<&CommandDataOptionValue>) -> Result<Self>;
}

macro_rules! impl_command_option {
    ($($Variant:ident($($Ty:ty),* $(,)?)),* $(,)?) => {
        $($(
            impl CommandOption for $Ty {
                const TYPE: CommandOptionType = CommandOptionType::$Variant;

                fn to_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption {
                    CreateCommandOption::new(Self::TYPE, name, description)
                        .required(true)
                }

                fn from_option(option: Option<&CommandDataOptionValue>) -> Result<Self> {
                    let option = option.ok_or(Error::MissingRequiredCommandOption)?;

                    match option {
                        CommandDataOptionValue::$Variant(v) => Ok(v.clone() as _),
                        _ => Err(Error::IncorrectCommandOptionType {
                            got: option.kind(),
                            expected: Self::TYPE,
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
            impl CommandOption for $Ty {
                const TYPE: CommandOptionType = CommandOptionType::Number;

                fn to_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption {
                    CreateCommandOption::new(Self::TYPE, name, description)
                        .required(true)
                        .min_number_value(<$Ty>::MIN as _)
                        .max_number_value(<$Ty>::MAX as _)
                }

                fn from_option(option: Option<&CommandDataOptionValue>) -> Result<Self> {
                    let option = option.ok_or(Error::MissingRequiredCommandOption)?;

                    match option {
                        CommandDataOptionValue::Number(v) => Ok(*v as _),
                        _ => Err(Error::IncorrectCommandOptionType {
                            got: option.kind(),
                            expected: Self::TYPE,
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
            impl CommandOption for $Ty {
                const TYPE: CommandOptionType = CommandOptionType::Integer;

                fn to_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption {
                    CreateCommandOption::new(Self::TYPE, name, description)
                        .required(true)
                        .min_int_value(<$Ty>::MIN as _)
                        .max_int_value(<$Ty>::MAX as _)
                }

                fn from_option(option: Option<&CommandDataOptionValue>) -> Result<Self> {
                    let option = option.ok_or(Error::MissingRequiredCommandOption)?;

                    match option {
                        CommandDataOptionValue::Integer(v) => Ok(*v as _),
                        _ => Err(Error::IncorrectCommandOptionType {
                            got: option.kind(),
                            expected: Self::TYPE,
                        }),
                    }
                }
            }
        )*
    };
}

impl_integer_command_option!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl<T: CommandOption> CommandOption for Option<T> {
    const TYPE: CommandOptionType = T::TYPE;

    fn to_option(name: impl Into<String>, description: impl Into<String>) -> CreateCommandOption {
        T::to_option(name, description).required(false)
    }

    fn from_option(option: Option<&CommandDataOptionValue>) -> Result<Self> {
        option
            .map(|option| T::from_option(Some(option)))
            .transpose()
    }
}
