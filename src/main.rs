use rand::random_range;
use serenity::{
    all::{
        Client, Context, CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, CreateMessage,
        EventHandler, GatewayIntents, Message, Timestamp,
        colours::roles::{DARK_GREEN, DARK_RED},
    },
    async_trait,
    model::gateway::Ready,
};
use sqlx::{Connection, SqliteConnection};
use std::{env, fs::File};

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
enum Item {
    LeafHandful,
    LeafPile,
    LeafBucket,
    LeafBarrel,
    LeafTruckload,
}

impl Item {
    fn as_str(&self) -> &'static str {
        match self {
            Item::LeafHandful => "Handful of Leaves",
            Item::LeafPile => "Pile of Leaves",
            Item::LeafBucket => "Bucket of Leaves",
            Item::LeafBarrel => "Barrel of Leaves",
            Item::LeafTruckload => "Truckload of Leaves",
        }
    }
}

async fn try_create_tables(conn: &mut SqliteConnection) {
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS user (
            id         INTEGER PRIMARY KEY,
            exp        INTEGER NOT NULL DEFAULT 0,
            leaves     INTEGER NOT NULL DEFAULT 0,
            last_raked INTEGER NOT NULL DEFAULT 0,
            last_daily INTEGER NOT NULL DEFAULT 0
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

async fn update_raking(
    user_id: i64,
    exp: i32,
    leaves: i32,
    last_raked: i64,
    conn: &mut SqliteConnection,
) {
    sqlx::query("UPDATE user SET exp = exp + ?, leaves = leaves + ?, last_raked = ? WHERE id = ?")
        .bind(exp)
        .bind(leaves)
        .bind(last_raked)
        .bind(user_id)
        .execute(conn)
        .await
        .unwrap();
}

async fn add_item(user_id: i64, item_id: i32, conn: &mut SqliteConnection) {
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

async fn raking(msg: &Message, builder: CreateMessage, risky: bool) -> CreateMessage {
    let mut conn = SqliteConnection::connect("sqlite:///data/rake.db")
        .await
        .expect("Couldn't connect to Rake's DB");
    let user_id = msg.author.id.get() as i64;
    try_register(user_id, &mut conn).await;
    let last_raked = get_last_raked(user_id, &mut conn).await;
    let current_time = msg.timestamp.unix_timestamp();
    let (delay, exp_range, leaves_range, remark) = if risky {
        (60, -10..40, -10..20, ", but with great risk")
    } else {
        (30, 5..10, 1..4, "")
    };
    let next_time = last_raked + delay;
    let exp = random_range(exp_range);
    let leaves = random_range(leaves_range);
    if next_time > current_time {
        builder.content(format!(
            "Your rake is on cooldown, you may try again <t:{next_time}:R>.",
        ))
    } else {
        update_raking(user_id, exp, leaves, current_time, &mut conn).await;
        let mut embed = CreateEmbed::new()
            .title(format!("You raked with `bare hands`{remark}.")) // change later
            .description(format!("`{exp:+} exp`\n`{leaves:+} leaves`"))
            .color(DARK_GREEN);
        if let Some(item) = match random_range(0..10000) {
            q if q < 500 => Some(Item::LeafHandful),   // 5%
            q if q < 600 => Some(Item::LeafPile),      // 1%
            q if q < 625 => Some(Item::LeafBucket),    // .25%
            q if q < 630 => Some(Item::LeafBarrel),    // .05%
            q if q < 631 => Some(Item::LeafTruckload), // .01%
            _ => None,
        } {
            embed = embed.field(
                "Bonus",
                format!(
                    "You also found a `{}`!\n*It is now in your inventory.*",
                    item.as_str()
                ),
                true,
            );
            add_item(user_id, item as i32, &mut conn).await;
        }
        builder.embed(embed)
    }
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
                        ("Raking", "`rake (r)`, `riskyRake (rr)`, `daily`, `rank`, `leaderboard (lb)`, `shop`, `inventory (inv)`, `character (char)`, `equip`, `unequip`, `info`, `sell`, `arena (pvp)`", false),
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
                "rake" | "r" => {
                    raking(&msg, builder, false).await
                }
                "riskyRake" | "rr" => {
                    raking(&msg, builder, true).await
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

    let _ = File::create_new("/data/rake.db"); // Only create if DB doesn't already exist
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
