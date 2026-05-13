use rand::random_range;
use serenity::all::{
    Client, Context, CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, CreateMessage,
    EventHandler, GatewayIntents, Message, Timestamp,
    colours::roles::{DARK_GREEN, DARK_RED},
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use sqlx::{Connection, SqliteConnection};
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
enum Items {
    LeafHandful,
    LeafPile,
    LeafBucket,
    LeafBarrel,
    LeafTruckload,
}

async fn try_create_tables(conn: &mut SqliteConnection) {
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS user (
            id         INTEGER PRIMARY KEY,
            exp        INTEGER NOT NULL DEFAULT 0,
            leaves     INTEGER NOT NULL DEFAULT 0,
            last_raked INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS item (
            id   INTEGER PRIMARY KEY,
            name TEXT    NOT NULL
        )",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS inventory (
            user_id  INTEGER NOT NULL,
            item_id  INTEGER NOT NULL,
            quantity INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, item_id),
            FOREIGN KEY (user_id) REFERENCES user(id),
            FOREIGN KEY (item_id) REFERENCES item(id)
        )",
    )
    .execute(conn)
    .await
    .unwrap();
}

async fn try_register(user_id: i64, conn: &mut SqliteConnection) {
    sqlx::query("INSERT OR IGNORE INTO user (id) VALUES (?)")
        .bind(user_id)
        .execute(conn)
        .await
        .unwrap();
}

async fn get_last_raked(user_id: i64, conn: &mut SqliteConnection) -> i64 {
    sqlx::query_as::<_, (i64,)>("SELECT last_raked FROM user WHERE id = ?")
        .bind(user_id)
        .fetch_one(conn)
        .await
        .unwrap()
        .0
}

async fn update_raking(user_id: i64, exp: u32, leaves: u32, conn: &mut SqliteConnection) {
    sqlx::query("UPDATE user SET exp = exp + ?, leaves = leaves + ? WHERE id = ?")
        .bind(exp)
        .bind(leaves)
        .bind(user_id)
        .execute(conn)
        .await
        .unwrap();
}

async fn add_item(user_id: i64, item_id: u32, conn: &mut SqliteConnection) {
    sqlx::query("INSERT OR IGNORE INTO inventory (user_id, item_id, quantity) VALUES (?, ?, 0)")
        .bind(user_id)
        .bind(item_id)
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query("UPDATE inventory SET quantity = quantity + 1 WHERE user_id = ? AND item_id = ?")
        .bind(user_id)
        .bind(item_id)
        .execute(conn)
        .await
        .unwrap();
}

fn fnv1a_hash(s: &str) -> usize {
    s.bytes().fold(0xcbf29ce484222325, |acc, b| {
        (acc ^ b as usize) * 0x100000001b3
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
        let content = match msg
            .content
            .strip_prefix("oi ")
            .or_else(|| msg.content.strip_prefix("Oi "))
        {
            Some(rest) => rest,
            None => return,
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
                "rake" => {
                    let mut conn = SqliteConnection::connect("sqlite:///data/rake.db")
                        .await.expect("Couldn't connect to Rake's DB");
                    let user_id = msg.author.id.get() as i64;
                    try_register(user_id, &mut conn).await;
                    let last_raked = get_last_raked(user_id, &mut conn).await;
                    if last_raked + 30 > msg.timestamp.unix_timestamp() {
                        builder.content(format!("Your rake is on cooldown, please wait **{}** more seconds.", 30 - (msg.timestamp.unix_timestamp() - last_raked)))
                    } else {
                        let exp = random_range(5..10);
                        let leaves = random_range(1..4);
                        update_raking(user_id, exp, leaves, &mut conn).await;
                        let mut embed = CreateEmbed::new()
                            .title("You raked with `bare hands`.") // change later
                            .description(format!("`+{exp} exp`\n`+{leaves} leaves`"))
                            .color(DARK_GREEN);
                        if let Some((gift, item)) = match random_range(0..10000) {
                            q if q < 500 => Some(("Handful of Leaves", Items::LeafHandful)), // 5%
                            q if q < 600 => Some(("Pile of Leaves", Items::LeafPile)), // 1%
                            q if q < 625 => Some(("Bucket of Leaves", Items::LeafBucket)), // .25%
                            q if q < 630 => Some(("Barrel of Leaves", Items::LeafBarrel)), // .05%
                            q if q < 631 => Some(("Truckload of Leaves", Items::LeafTruckload)), // .01%
                            _ => None
                        } {
                            add_item(user_id, item as u32, &mut conn).await;
                            embed = embed.field("Bonus", format!("You also found a `{gift}`!\n*It is now in your inventory.*"), true)
                        }
                        builder.embed(embed)
                    }
                }
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

    let mut conn = SqliteConnection::connect("sqlite:///data/rake.db")
        .await
        .expect("Couldn't connect to Rake's DB");
    try_create_tables(&mut conn).await;

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
