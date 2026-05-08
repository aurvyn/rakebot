use serenity::all::{
    Client, Context, CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, CreateMessage,
    EventHandler, GatewayIntents, Message, Timestamp,
    colours::roles::{DARK_GREEN, DARK_RED},
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use std::env;

const ICON_URL: &str = "https://img.icons8.com/emoji/452/fallen-leaf.png";
const RESPONSES: &[&str] = &[
    "Yes",
    "Maybe",
    "No",
    "You're dumb to ask that question",
    "I think you already know the answer",
    "`Error: Response Too Dumb`",
    "Yup",
    "Nay",
    "I feel hurt hearing that...",
    "Yes but actually no.",
    "Why is that a question? The answer is obviously YES!",
    "...so you're a Rake of culture as well!",
    "POGGERS",
    "That made me want to eat some wasabi out of an ice cream cone.",
    "That's some bold question you're asking.",
    "Yesn't",
    "Will you be mad if I say no?",
    "It's a tough question... hmm... I'll say yes.",
    "Definitely!",
    "Nahhh",
];

fn fnv1a_hash(s: &str) -> usize {
    s.bytes().fold(0xcbf29ce484222325, |acc, b| {
        acc ^ b as usize * 0x100000001b3
    })
}

trait Choice<T> {
    fn choice(&self, seed: &str) -> &T;
}

impl<T> Choice<T> for [T] {
    fn choice(&self, seed: &str) -> &T {
        &self[fnv1a_hash(seed) % self.len()]
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let content = match msg.content.strip_prefix("oi ") {
            Some(rest) => rest,
            None => match msg.content.strip_prefix("Oi ") {
                Some(rest) => rest,
                None => return,
            },
        };
        let mut builder = CreateMessage::new();
        builder = match content.split_once(" ") {
            Some((command, input)) => match command {
                "say" | "say," => builder.embed(CreateEmbed::new()
                    .title("Question")
                    .description(input)
                    .color(DARK_GREEN)
                    .field("Answer", *RESPONSES.choice(input), true)),
                "speak" => builder.content(input).allowed_mentions(CreateAllowedMentions::new()),
                _ => builder.embed(CreateEmbed::new()
                    .title(format!("What's `{command}`?"))
                    .description("I can't quite understand what you're saying, maybe try `oi help`?")
                    .color(DARK_RED))
            }
            None => match content {
                "help" => builder.embed(CreateEmbed::new()
                    .title("Commands")
                    .description("[Join our official server!](https://discord.gg/fwNnyndEM2)")
                    .color(DARK_GREEN)
                    .thumbnail(ICON_URL)
                    .fields(vec![
                        ("Raking", "`rake`, `riskyRake`, `daily`, `rank`, `leaderboard`, `shop`, `inventory`, `character`, `equip`, `unequip`, `info`, `sell`, `pvp`", false),
                        ("Fun", "`say`", false),
                        ("Utility", "`ping`, `invite`", false),
                        ("Music", "`play`, `leave`", false),
                        ("Admin", "`speak`, `settings`", false),
                    ])
                    .footer(CreateEmbedFooter::new("yee haw").icon_url(ICON_URL))),
                "ping" => builder.embed(CreateEmbed::new()
                    .title("🏓 Pong!")
                    .color(DARK_GREEN)
                    .timestamp(Timestamp::now())),
                _ => builder.embed(CreateEmbed::new()
                    .title("What?")
                    .description("I can't quite understand what you're saying, maybe try `oi help`?")
                    .color(DARK_RED))
            }
        };
        if let Err(why) = msg.channel_id.send_message(&ctx.http, builder).await {
            println!("Error sending message: {why:?}");
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("RAKE_TOKEN").expect("Expected Rake's token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
