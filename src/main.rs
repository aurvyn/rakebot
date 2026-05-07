use serenity::all::{
    Client, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, EventHandler, GatewayIntents,
    Message, Timestamp,
    colours::roles::{DARK_GREEN, DARK_RED},
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use std::env;

const ICON_URL: &str = "https://img.icons8.com/emoji/452/fallen-leaf.png";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let content = match msg.content.strip_prefix("oi ") {
            Some(rest) => rest,
            None => return,
        };
        let embed = match content {
            "help" => {
                CreateEmbed::new()
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
                    .footer(CreateEmbedFooter::new("yee haw").icon_url(ICON_URL))
            }
            "ping" => {
                CreateEmbed::new()
                    .title("🏓 Pong!")
                    .color(DARK_GREEN)
            }
            _ => {
                CreateEmbed::new()
                    .title("What?")
                    .description("I can't quite understand what you're saying, maybe try `oi help`?")
                    .color(DARK_RED)
            }
        }.timestamp(Timestamp::now());
        let builder = CreateMessage::new().embed(embed);
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
    let token = env::var("RAKE_TOKEN").expect("Expected a token in the environment");
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
