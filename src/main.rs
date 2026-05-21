use rand::random_range;
use serenity::{
    all::{
        Client, Context, CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, CreateMessage,
        EventHandler, GatewayIntents, Message, Ready, Timestamp, UserId,
        colours::roles::{DARK_GREEN, DARK_RED},
    },
    async_trait,
    futures::StreamExt,
    prelude::TypeMapKey,
};
use sqlx::SqlitePool;
use std::{env, fs::File};

const ICON_URL: &str = "https://img.icons8.com/emoji/452/fallen-leaf.png";
const GIFT_DROPCHANCE: &str = "- **`Handful of Leaves`**: Grants 20 Leaves upon selling (5%)
- **`Pile of Leaves`**: Grants 100 Leaves upon selling (1%)
- **`Bucket of Leaves`**: Grants 400 Leaves upon selling (0.25%)
- **`Barrel of Leaves`**: Grants 2,000 Leaves upon selling (0.05%)
- **`Truckload of Leaves`**: Grants 10,000 Leaves upon selling (0.01%)";
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

macro_rules! get_pool {
    ($ctx:expr) => {
        $ctx.data.read().await.get::<DbPool>().unwrap()
    };
}

enum RakeType {
    Normal,
    Risky,
    Daily,
}

enum Item {
    LeafHandful,
    LeafPile,
    LeafBucket,
    LeafBarrel,
    LeafTruckload,
}

impl Item {
    fn from(id: i32) -> Option<Self> {
        match id {
            v if v == Item::LeafHandful as i32 => Some(Item::LeafHandful),
            v if v == Item::LeafPile as i32 => Some(Item::LeafPile),
            v if v == Item::LeafBucket as i32 => Some(Item::LeafBucket),
            v if v == Item::LeafBarrel as i32 => Some(Item::LeafBarrel),
            v if v == Item::LeafTruckload as i32 => Some(Item::LeafTruckload),
            _ => None,
        }
    }

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
struct DbPool;
impl TypeMapKey for DbPool {
    type Value = SqlitePool;
}

async fn try_create_tables(pool: &SqlitePool) {
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
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS inventory (
            user_id  INTEGER NOT NULL,
            item_id  INTEGER NOT NULL,
            quantity INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, item_id),
            FOREIGN KEY (user_id) REFERENCES user(id)
        )",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn try_register(user_id: i64, pool: &SqlitePool) {
    sqlx::query(&format!(
        "INSERT OR IGNORE INTO user (id) VALUES ({user_id})"
    ))
    .execute(pool)
    .await
    .unwrap();
}

async fn get_from_user(field: &str, user_id: i64, pool: &SqlitePool) -> i64 {
    let (result,) = sqlx::query_as(&format!("SELECT {field} FROM user WHERE id = {user_id}"))
        .fetch_one(pool)
        .await
        .unwrap();
    result
}

async fn get_inventory(user_id: i64, pool: &SqlitePool) -> Vec<(i32, i32)> {
    sqlx::query_as(&format!(
        "SELECT item_id, quantity FROM inventory WHERE user_id = {user_id}"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn get_lb(pool: &SqlitePool, server_ids: Vec<u64>, limit: Option<u8>) -> Vec<(u64, i64)> {
    sqlx::query_as(&format!(
        "SELECT id, exp FROM user {} ORDER BY exp DESC, leaves DESC {}",
        if server_ids.is_empty() {
            String::new()
        } else {
            format!(
                "WHERE id IN ({})",
                server_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        },
        if let Some(lim) = limit {
            format!("LIMIT {lim}")
        } else {
            String::new()
        }
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn update_raking(
    user_id: i64,
    exp: i32,
    leaves: i32,
    field: &str,
    last_raked: i64,
    pool: &SqlitePool,
) {
    sqlx::query(&format!(
        "UPDATE user SET exp = exp + {exp}, leaves = leaves + {leaves}, {field} = {last_raked} WHERE id = {user_id}"
    ))
    .execute(pool)
    .await
    .unwrap();
}

async fn add_item(user_id: i64, item_id: i32, pool: &SqlitePool) {
    sqlx::query(&format!(
        "INSERT OR IGNORE INTO inventory (user_id, item_id, quantity) VALUES ({user_id}, {item_id}, 0)"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE inventory SET quantity = quantity + 1 WHERE user_id = {user_id} AND item_id = {item_id}"
    ))
    .execute(pool)
    .await
    .unwrap();
}

async fn raking(
    ctx: &Context,
    msg: &Message,
    builder: CreateMessage,
    rake_type: RakeType,
) -> CreateMessage {
    let user_id = msg.author.id.get() as i64;
    try_register(user_id, get_pool!(ctx)).await;
    let (delay, field, exp_range, leaves_range, remark, method) = match rake_type {
        RakeType::Normal => (
            30,
            "last_raked",
            5..10,
            1..4,
            "You raked",
            "Raking is on cooldown",
        ),
        RakeType::Risky => (
            60,
            "last_raked",
            -10..40,
            -6..16,
            "With great risk, you raked",
            "Risky raking is on cooldown",
        ),
        RakeType::Daily => (
            72000, // 20 hours
            "last_daily",
            200..240,
            100..120,
            "You claimed your daily reward by raking",
            "You already claimed your daily reward",
        ),
    };
    let time = msg.timestamp.unix_timestamp();
    let next_time = get_from_user(field, user_id, get_pool!(ctx)).await + delay;
    if next_time > time {
        return builder.content(format!("{method}, please try again <t:{next_time}:R>.",));
    }
    let exp = random_range(exp_range);
    let leaves = random_range(leaves_range);
    update_raking(user_id, exp, leaves, field, time, get_pool!(ctx)).await;
    let mut embed = CreateEmbed::new()
        .title(format!("{remark} with `bare hands`.")) // change later
        .description(format!("`{exp:+} exp`\n`{leaves:+} Leaves`"))
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
        add_item(user_id, item as i32, get_pool!(ctx)).await;
    }
    builder.embed(embed)
}

async fn get_lb_string(ctx: &Context, server_ids: Vec<u64>) -> String {
    let mut top10 = String::new();
    for (id, exp) in get_lb(get_pool!(ctx), server_ids, Some(10)).await {
        top10 += &if let Ok(user) = UserId::new(id).to_user(&ctx).await {
            format!("1. `{:<20}{exp:>8} exp`\n", user.name)
        } else {
            format!("1. `{:<20}{exp:>8} exp`\n", "???")
        }
    }
    top10
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
                "help" => match input {
                    "rake" | "r" => builder.embed(CreateEmbed::new()
                        .title("`rake` (alias: `r`)")
                        .description("The basic command to rake and obtain Leaves.")
                        .color(DARK_GREEN)
                        .field("Details", "**exp**:\n`5`-`10`\n\
                            **Leaves**:\n`(1 + 0.1 * <strength>)`-`(4 + 0.5 * <rake size> * <rake efficiency>)`\n\
                            **cooldown**:\n`(30 + <rake size>)` seconds", false)
                        .field("Drops", GIFT_DROPCHANCE, false)),
                    "riskyRake" | "rr" => builder.embed(CreateEmbed::new()
                        .title("`riskyRake` (alias: `rr`)")
                        .description("The risky version of raking to obtain or lose Leaves.")
                        .color(DARK_GREEN)
                        .field("Details", "**exp**:\n`-10`-`40`\n\
                            **Leaves**:\n`(-6 + 0.2 * <strength>)`-`(16 + <rake size> * <rake efficiency>)`\n\
                            **cooldown**:\n`(60 + 2 * <rake size>)` seconds", false)
                        .field("Drops", GIFT_DROPCHANCE, false)),
                    _ => builder.embed(CreateEmbed::new()
                        .title(format!("What's `{input}`?"))
                        .description("I don't have that command, try `oi help` to see available commands.")
                        .color(DARK_RED))
                }
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
                        ("Raking", "`rake (r)`, `riskyRake (rr)`, `daily`, `rank`,\
                            `leaderboard (lb)`, `shop`, `inventory (inv)`, `character (char)`,\
                            `equip`, `unequip`, `info`, `sell`, `arena (pvp)`", false),
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
                    raking(&ctx, &msg, builder, RakeType::Normal).await
                }
                "riskyRake" | "rr" => {
                    raking(&ctx, &msg, builder, RakeType::Risky).await
                }
                "daily" => {
                    raking(&ctx, &msg, builder, RakeType::Daily).await
                }
                "inventory" | "inv" => {
                    let user_id = msg.author.id.get() as i64;
                    builder.embed(CreateEmbed::new()
                    .title("Your inventory")
                    .description(get_inventory(user_id, get_pool!(ctx)).await
                        .into_iter().map(|(item_id, quantity)|
                            format!("{quantity} of {}", Item::from(item_id).unwrap().as_str()))
                        .collect::<Vec<_>>().join("\n")
                        + &format!("\n\n`Your Leaves: {}`", get_from_user("leaves", user_id, get_pool!(ctx)).await))
                    .color(DARK_GREEN))
                }
                "leaderboard" | "lb" => {
                    let loading_msg = msg.channel_id.send_message(&ctx.http,
                        CreateMessage::new().content("<a:loading:822855468731990058> loading...")
                        ).await.unwrap_or_default();
                    let top10global = get_lb_string(&ctx, vec![]).await;
                    let mut embed = CreateEmbed::new()
                        .title("Global Leaderboard")
                        .description(top10global)
                        .color(DARK_GREEN);
                    let mut server_ids = vec![];
                    if let Some(guild_id) = msg.guild_id {
                        let mut members = guild_id.members_iter(&ctx).boxed();
                        while let Some(result) = members.next().await {
                            if let Ok(member) = result {
                                server_ids.push(member.user.id.get());
                            }
                        }
                        let top10server = get_lb_string(&ctx, server_ids).await;
                        embed = embed.field("Server Leaderboard", top10server, false);
                    }
                    loading_msg.delete(&ctx).await.unwrap();
                    builder.embed(embed)
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
    let pool = SqlitePool::connect(
        &env::var("DATABASE_URL").expect("Expected Rake's database url in the environment"),
    )
    .await
    .expect("Couldn't connect to Rake's DB");
    try_create_tables(&pool).await;
    {
        let mut data = client.data.write().await;
        data.insert::<DbPool>(pool);
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
