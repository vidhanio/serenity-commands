#![allow(missing_docs, dead_code)]

use serenity::all::{
    async_trait, AutocompleteChoice, Client, Context, CreateAutocompleteResponse,
    CreateInteractionResponse, CreateInteractionResponseMessage, EventHandler, GatewayIntents,
    GuildId, Interaction,
};
use serenity_commands::{Autocomplete, AutocompleteCommands, Command, Commands};

const WORDS: &str = include_str!("words_alpha.txt");

#[derive(Debug, Commands)]
#[allow(clippy::enum_variant_names)]
enum AllCommands {
    /// Get the index of a word in the dictionary.
    #[command(autocomplete)]
    GetWordIndex(GetWordIndexCommand),
}

impl AllCommands {
    fn run(self) -> String {
        match self {
            Self::GetWordIndex(get_word_index) => get_word_index.run(),
        }
    }
}

#[derive(Debug, Command)]
struct GetWordIndexCommand {
    /// The end of the word to find the index of.
    suffix: String,

    /// The word to find the index of.
    #[command(autocomplete)]
    word: String,
}

impl GetWordIndexCommand {
    fn run(self) -> String {
        let words = WORDS.lines().collect::<Vec<_>>();
        let index = words
            .iter()
            .position(|word| word == &self.word)
            .map_or(0, |index| index + 1);
        index.to_string()
    }
}

impl GetWordIndexCommandAutocomplete {
    fn autocomplete(self) -> CreateAutocompleteResponse {
        let Self::Word { suffix, word } = self;

        CreateAutocompleteResponse::new().set_choices(
            WORDS
                .lines()
                .filter(|w| {
                    w.starts_with(&word)
                        && w.ends_with(&suffix.as_inner().cloned().unwrap_or_default())
                })
                .map(|w| AutocompleteChoice::new(w, w))
                .take(25)
                .collect(),
        )
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
        match interaction {
            Interaction::Command(command) => {
                let command_data = AllCommands::from_command_data(&command.data).unwrap();
                command
                    .create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(command_data.run())
                                .ephemeral(true),
                        ),
                    )
                    .await
                    .unwrap();
            }
            Interaction::Autocomplete(autocomplete) => {
                let autocomplete_data =
                    Autocomplete::<AllCommands>::from_command_data(&autocomplete.data).unwrap();
                let response = match autocomplete_data {
                    AllCommandsAutocomplete::GetWordIndex(get_word_index) => {
                        get_word_index.autocomplete()
                    }
                };
                autocomplete
                    .create_response(ctx, CreateInteractionResponse::Autocomplete(response))
                    .await
                    .unwrap();
            }
            _ => (),
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
