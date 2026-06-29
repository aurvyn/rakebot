use crate::{BaseItem::*, Limb::*};
use rand::{SeedableRng, random_bool, random_range, rngs::StdRng, seq::IndexedRandom};
use serenity::{
    all::{
        Client, Context, CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, CreateMessage,
        EventHandler, GatewayIntents, Message, Ready, Timestamp, UserId,
        colours::css::{DANGER, POSITIVE, WARNING},
    },
    async_trait,
    futures::StreamExt,
    prelude::TypeMapKey,
};
use sqlx::SqlitePool;
use std::{cmp::min, env, fs::File, str::FromStr};

type ItemId = u32;
type PassiveId = u32;
type QualityId = u8;
type ModifierId = u8;

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
const QUALITY: &[&str] = &["🌑", "🌘", "🌗", "🌖", "🌕"];

macro_rules! get_pool {
    ($ctx:expr) => {
        $ctx.data.read().await.get::<DbPool>().unwrap()
    };
}

macro_rules! get_owner_id {
    ($ctx:expr) => {
        $ctx.data.read().await.get::<OwnerId>().unwrap()
    };
}

enum RakeType {
    Normal,
    Risky,
    Daily,
}

enum Limb {
    Head,
    Neck,
    Torso,
    LeftUpperArm,
    RightUpperArm,
    LeftLowerArm,
    RightLowerArm,
    LeftHand,
    RightHand,
    LeftUpperLeg,
    RightUpperLeg,
    LeftLowerLeg,
    RightLowerLeg,
    LeftFoot,
    RightFoot,
}

#[derive(Clone, Copy, strum::AsRefStr, strum::EnumString, strum::FromRepr)]
enum Modifier {
    Normal,
    Lousy,
    Dull,
    Light,
    Heavy,
    Bulky,
    Intimidating,
    Superior,
    Legendary,
}

#[derive(Clone, Copy, PartialEq, strum::AsRefStr, strum::EnumString, strum::FromRepr)]
enum BaseItem {
    #[strum(to_string = "Handful of Leaves")]
    LeafHandful,
    #[strum(to_string = "Pile of Leaves")]
    LeafPile,
    #[strum(to_string = "Bucket of Leaves")]
    LeafBucket,
    #[strum(to_string = "Barrel of Leaves")]
    LeafBarrel,
    #[strum(to_string = "Truckload of Leaves")]
    LeafTruckload,
    #[strum(to_string = "Tank Top")]
    TankTop,
    #[strum(to_string = "T-Shirt")]
    Tshirt,
    Sneaker,
    Shorts,
    #[strum(to_string = "Baseball Cap")]
    BaseballCap,
    #[strum(to_string = "Propeller Hat")]
    PropellerHat,
    Sweater,
    Hoodie,
    Jacket,
    Coat,
    #[strum(to_string = "Latex Glove")]
    LatexGlove,
    #[strum(to_string = "Cotton Glove")]
    CottonGlove,
    #[strum(to_string = "Leather Glove")]
    LeatherGlove,
    #[strum(to_string = "Arm Sleeve")]
    ArmSleeve,
    #[strum(to_string = "Leg Sleeve")]
    LegSleeve,
    #[strum(to_string = "Paper Knife")]
    PaperKnife,
    #[strum(to_string = "Tree Stick")]
    TreeStick,
    #[strum(to_string = "Brass Knuckles")]
    BrassKnuckles,
    #[strum(to_string = "Spiked Brass Knuckles")]
    SpikedBrassKnuckles,
    #[strum(to_string = "Bladed Brass Knuckles")]
    BladedBrassKnuckles,
    Orangeberries,
    #[strum(to_string = "Pen Pineapple Apple Pen")]
    PenPineappleApplePen,
    #[strum(to_string = "Health Potion")]
    HealthPotion,
    #[strum(to_string = "Instant Ramen")]
    InstantRamen,
    Ramen,
    #[strum(to_string = "Carolina Reaper Ramen")]
    CarolinaReaperRamen,
    Milk,
    #[strum(to_string = "Fried Snow")]
    FriedSnow,
    Scarf,
}

impl BaseItem {
    fn description(&self) -> &'static str {
        match self {
            LeafHandful => "It's a handful of leaves! Mind if I take some?",
            LeafPile => "Are you really just gonna ***leave*** them on the ground like that?",
            LeafBucket => "A bucket o' leaves.",
            LeafBarrel => "Currency in an alcohol container.",
            LeafTruckload => "So much leaves that you can swim in them!",
            TankTop => "Does not come with a turret.",
            Tshirt => "A normal T-shirt. Luckily, you don't have to wash it everyday.",
            Sneaker => "A shoe for your foot. Nothing remarkable.",
            Shorts => "Too short for you? That's too bad.",
            BaseballCap => "No cap, this is a cap.",
            PropellerHat => "\"Become an attack helicopter today!\"",
            Sweater => "A pink sweater, made by a grandma's love and care.",
            Hoodie => "Gangster enough to give you some fighting spirit.",
            Jacket => "Who is Jack why are they et.",
            Coat => "Oh wait, that diamond on the zipper is fake...",
            LatexGlove => "You're not going to use it more than once, are you...?",
            CottonGlove => "Perfect for the winter season.",
            LeatherGlove => "Gangsta.",
            ArmSleeve => "Your best companion for normal workouts.",
            LegSleeve => "Prevent shin splits",
            PaperKnife => "Don't underestimate the power of paper cuts.",
            TreeStick => {
                "Instead of using it as fire fuel, you're gonna swing it around like a madman, aren't you?"
            }
            BrassKnuckles => "Dangerous if used in the wrong hands",
            SpikedBrassKnuckles => "Quite spikey.",
            BladedBrassKnuckles => "Punch and cut at the same time. Win-win.",
            Orangeberries => "Tastes exactly the opposite of blueberries.",
            PenPineappleApplePen => {
                "I don't have a pen\nI don't have an apple\nno apple pen!\nI don't have a pen\nI don't have pineapple\nno pineapple pen!\nNo apple pen, no pineaple pen, no pen pineapple apple pen!\nNo pen pineapple apple pen!"
            }
            HealthPotion => "A classic consumable item.",
            InstantRamen => "Don't tell me you're going to eat it dry?",
            Ramen => "Steaming hot, but not spicy hot.",
            CarolinaReaperRamen => "You have a death wish if you want to eat this abomination.",
            Milk => "Dad went to get milk, but never came back. Did you see him on the way?",
            FriedSnow => "It's got too much sentimental value to be eaten... or does it?",
            Scarf => "Scarfing down a snowman?",
        }
    }

    fn buying_price(&self) -> i32 {
        match self {
            LeafHandful => 20,
            LeafPile => 100,
            LeafBucket => 400,
            LeafBarrel => 2000,
            LeafTruckload => 10000,
            TankTop => 10,
            Tshirt => 10,
            Sneaker => 20,
            Shorts => 30,
            BaseballCap => 10,
            PropellerHat => 50,
            Sweater => 100,
            Hoodie => 200,
            Jacket => 450,
            Coat => 1999,
            LatexGlove => 1,
            CottonGlove => 30,
            LeatherGlove => 50,
            ArmSleeve => 25,
            LegSleeve => 30,
            PaperKnife => 5,
            TreeStick => 20,
            BrassKnuckles => 150,
            SpikedBrassKnuckles => 400,
            BladedBrassKnuckles => 999,
            Orangeberries => 20,
            PenPineappleApplePen => 50,
            HealthPotion => 100,
            InstantRamen => 5,
            Ramen => 150,
            CarolinaReaperRamen => 666,
            Milk => 250,
            FriedSnow => 99999,
            Scarf => 40,
        }
    }

    fn selling_price(&self) -> i32 {
        match self {
            LeafHandful => 20,
            LeafPile => 100,
            LeafBucket => 400,
            LeafBarrel => 2000,
            LeafTruckload => 10000,
            TankTop => 5,
            Tshirt => 5,
            Sneaker => 10,
            Shorts => 15,
            BaseballCap => 5,
            PropellerHat => 25,
            Sweater => 50,
            Hoodie => 100,
            Jacket => 200,
            Coat => 50,
            LatexGlove => 0,
            CottonGlove => 10,
            LeatherGlove => 20,
            ArmSleeve => 5,
            LegSleeve => 5,
            PaperKnife => 0,
            TreeStick => 0,
            BrassKnuckles => 100,
            SpikedBrassKnuckles => 300,
            BladedBrassKnuckles => 666,
            Orangeberries => 15,
            PenPineappleApplePen => 40,
            HealthPotion => 100,
            InstantRamen => 5,
            Ramen => 100,
            CarolinaReaperRamen => 444,
            Milk => 200,
            FriedSnow => -99999,
            Scarf => 15,
        }
    }

    fn equipable_limbs(&self) -> Vec<Limb> {
        match self {
            TankTop => vec![Torso],
            Tshirt => vec![Torso, LeftUpperArm, RightUpperArm],
            Sweater | Hoodie | Jacket | Coat => vec![
                Torso,
                LeftUpperArm,
                RightUpperArm,
                LeftLowerArm,
                RightLowerArm,
            ],
            Sneaker => vec![LeftFoot, RightFoot],
            Shorts => vec![LeftUpperLeg, RightUpperLeg],
            BaseballCap | PropellerHat => vec![Head],
            LatexGlove | CottonGlove | LeatherGlove => vec![LeftHand, RightHand],
            ArmSleeve => vec![LeftLowerArm, RightLowerArm],
            LegSleeve => vec![LeftLowerLeg, RightLowerLeg],
            Scarf => vec![Neck],
            _ => vec![],
        }
    }

    fn equipments() -> [Self; 16] {
        [
            TankTop,
            Tshirt,
            Sweater,
            Hoodie,
            Jacket,
            Coat,
            Sneaker,
            Shorts,
            BaseballCap,
            PropellerHat,
            LatexGlove,
            CottonGlove,
            LeatherGlove,
            ArmSleeve,
            LegSleeve,
            Scarf,
        ]
    }

    fn is_equipment(&self) -> bool {
        Self::equipments().contains(self)
    }

    fn weapons() -> [Self; 5] {
        [
            PaperKnife,
            TreeStick,
            BrassKnuckles,
            SpikedBrassKnuckles,
            BladedBrassKnuckles,
        ]
    }

    fn consumables() -> [Self; 8] {
        [
            Orangeberries,
            PenPineappleApplePen,
            HealthPotion,
            InstantRamen,
            Ramen,
            CarolinaReaperRamen,
            Milk,
            FriedSnow,
        ]
    }

    fn weight(&self) -> u32 {
        match self {
            TankTop => 64,
            Tshirt => 64,
            Sweater => 16,
            Hoodie => 16,
            Jacket => 8,
            Coat => 8,
            Sneaker => 16,
            Shorts => 16,
            BaseballCap => 8,
            PropellerHat => 2,
            LatexGlove => 16,
            CottonGlove => 8,
            LeatherGlove => 8,
            ArmSleeve => 8,
            LegSleeve => 8,
            PaperKnife => 64,
            TreeStick => 64,
            BrassKnuckles => 32,
            SpikedBrassKnuckles => 16,
            BladedBrassKnuckles => 8,
            Orangeberries => 64,
            PenPineappleApplePen => 16,
            HealthPotion => 32,
            InstantRamen => 32,
            Ramen => 16,
            CarolinaReaperRamen => 8,
            Milk => 64,
            FriedSnow => 4,
            Scarf => 8,
            _ => 0,
        }
    }
}

trait ShopRep {
    fn shop_rep(&self, start: usize) -> String;
}

impl ShopRep for Vec<BaseItem> {
    fn shop_rep(&self, start: usize) -> String {
        let mut result = String::new();
        for item in self {
            result += &format!(
                "{start}. `{:<24}{:>6} Leaves`\n",
                item.as_ref(),
                item.buying_price()
            )
        }
        result
    }
}

struct Item {
    base: BaseItem,
    quality: QualityId, // up to 20
    modifier: Modifier,
}

impl Item {
    fn from_base(base: BaseItem) -> Self {
        Item {
            base,
            quality: 0,
            modifier: Modifier::Normal,
        }
    }

    /// Format: `[modifier] <item> [quality]`
    fn from_str(s: &str) -> Option<Self> {
        Some(if let Some((modifier, inner)) = s.split_once(" ") {
            if let Some((item, quality)) = inner.rsplit_once(" ") {
                Self {
                    base: BaseItem::from_str(item).ok()?,
                    quality: quality.parse().ok()?,
                    modifier: Modifier::from_str(modifier).ok()?,
                }
            } else {
                // possibly no quality specified, assume 0
                Self {
                    base: BaseItem::from_str(inner).ok()?,
                    quality: 0,
                    modifier: Modifier::from_str(modifier).ok()?,
                }
            }
        } else {
            // possibly no modifier specified, assume normal
            Self::from_base(BaseItem::from_str(s).ok()?)
        })
    }

    fn full_name(&self) -> String {
        format!("{} {}", self.modifier.as_ref(), self.base.as_ref())
    }

    /// To be only used with equipments.
    /// Check if they are equipments with [`BaseItem::is_equipment`].
    fn with_quality(&self, quantity: u32) -> String {
        let mut quality = String::from("- ");
        for i in 0..4 {
            quality.push_str(
                QUALITY
                    .get(min(self.quality.saturating_sub(i * 5), 4) as usize)
                    .unwrap(),
            );
        }
        format!("{} {quality}", self.with_quantity(quantity))
    }

    fn with_quantity(&self, quantity: u32) -> String {
        format!("`{:<40}`", format!("{quantity} of {}", self.full_name()))
    }

    /// Prints out the formatted representation with
    /// quantity, modifier, and quality.
    fn as_owned(&self, quantity: u32) -> String {
        if self.base.is_equipment() {
            self.with_quality(quantity)
        } else {
            self.with_quantity(quantity)
        }
    }
}

fn sample_items(rng: &mut StdRng, amount: usize, items: &[BaseItem]) -> Vec<BaseItem> {
    items
        .sample_weighted(rng, amount, |item| item.weight())
        .unwrap()
        .cloned()
        .collect::<Vec<_>>()
}

fn get_shop(seed: u64) -> (Vec<BaseItem>, Vec<BaseItem>, Vec<BaseItem>) {
    let ref mut rng = StdRng::seed_from_u64(seed);
    let equipments = sample_items(rng, 4, &BaseItem::equipments());
    let weapons = sample_items(rng, 2, &BaseItem::weapons());
    let consumables = sample_items(rng, 3, &BaseItem::consumables());
    (equipments, weapons, consumables)
}

#[derive(strum::AsRefStr, strum::FromRepr)]
enum Passive {
    #[strum(to_string = "Lucky!")]
    Lucky,
    #[strum(to_string = "Luck o' Clock!")]
    LuckyZero,
    #[strum(to_string = "Unlucky...")]
    Unlucky,
}

impl Passive {
    fn description(&self) -> &'static str {
        match self {
            Passive::Lucky => "+10% to maximum possible Leaves received from raking (1 hour)",
            Passive::LuckyZero => "+10% chance to 10x Leaves received from raking (3 hours)",
            Passive::Unlucky => "-10% to minimum possible Leaves received from raking (10 minutes)",
        }
    }

    /// Returns duration of passive in seconds
    fn duration(&self) -> i64 {
        60 * match self {
            Passive::Lucky => 60,
            Passive::LuckyZero => 180,
            Passive::Unlucky => 10,
        }
    }
    /// (min leaves multiplier, max leaves multiplier, final multiplier, chance)
    fn modifiers(&self) -> (f64, f64, f64, f64) {
        match self {
            Passive::Lucky => (0., 0.1, 0., 1.),
            Passive::LuckyZero => (0., 0., 10., 0.1),
            Passive::Unlucky => (-0.1, 0., 0., 1.),
        }
    }
}

struct DbPool;
impl TypeMapKey for DbPool {
    type Value = SqlitePool;
}

struct OwnerId;
impl TypeMapKey for OwnerId {
    type Value = i64;
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
        CREATE TABLE IF NOT EXISTS item (
            user_id  INTEGER NOT NULL,
            item_id  INTEGER NOT NULL,
            quantity INTEGER NOT NULL DEFAULT 0,
            quality  INTEGER NOT NULL DEFAULT 0,
            modifier INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, item_id, quality, modifier),
            FOREIGN KEY (user_id) REFERENCES user(id)
        )",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS passive (
            user_id    INTEGER NOT NULL,
            passive_id INTEGER NOT NULL,
            expires_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, passive_id),
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

async fn get_passives(user_id: i64, time: i64, pool: &SqlitePool) -> Vec<(PassiveId, i64)> {
    sqlx::query_as(&format!(
        "SELECT passive_id, expires_at FROM passive WHERE user_id = {user_id} AND expires_at > {time}"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Returns the amount that the user owns.
async fn get_item(
    user_id: i64,
    item_id: ItemId,
    quality: QualityId,
    modifier: ModifierId,
    pool: &SqlitePool,
) -> Option<u32> {
    let quantity = sqlx::query_as(&format!(
        "SELECT quantity FROM item WHERE user_id = {user_id} AND item_id = {item_id} AND quality = {quality} AND modifier = {modifier}"
    ))
    .fetch_one(pool)
    .await
    .ok()
    .map(|(q,)| q);
    quantity
}

async fn get_items(user_id: i64, pool: &SqlitePool) -> Vec<(Item, u32)> {
    sqlx::query_as(&format!(
        "SELECT item_id, quantity, quality, modifier FROM item WHERE user_id = {user_id}"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
    .iter()
    .map(|(item_id, quantity, quality, modifier)| {
        let item_id: ItemId = *item_id;
        let modifier_id: ModifierId = *modifier;
        (
            Item {
                base: BaseItem::from_repr(item_id as usize).unwrap(),
                quality: *quality,
                modifier: Modifier::from_repr(modifier_id as usize).unwrap(),
            },
            *quantity,
        )
    })
    .collect()
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

async fn add_item(user_id: i64, item: Item, pool: &SqlitePool) {
    let item_id = item.base as ItemId;
    sqlx::query(&format!(
        "INSERT OR IGNORE INTO item (user_id, item_id, quantity) VALUES ({user_id}, {item_id}, 0)",
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE item SET quantity = quantity + 1 WHERE user_id = {user_id} AND item_id = {item_id}",
    ))
    .execute(pool)
    .await
    .unwrap();
}

async fn add_passive(user_id: i64, expires_at: i64, passive: Passive, pool: &SqlitePool) {
    let passive_id = passive as i32;
    sqlx::query(&format!(
        "INSERT OR IGNORE INTO passive (user_id, passive_id, expires_at) VALUES ({user_id}, {passive_id}, 0)"
    ))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE passive SET expires_at = {expires_at} WHERE user_id = {user_id} AND passive_id = {passive_id}"
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
    let (delay, field, exp_range, leaves_start, leaves_end, remark, method) = match rake_type {
        RakeType::Normal => (
            30,
            "last_raked",
            5..11,
            1.,
            5.,
            "You raked",
            "Raking is on cooldown",
        ),
        RakeType::Risky => (
            60,
            "last_raked",
            -10..41,
            -6.,
            17.,
            "With great risk, you raked",
            "Risky raking is on cooldown",
        ),
        RakeType::Daily => (
            72000, // 20 hours
            "last_daily",
            200..241,
            100.,
            121.,
            "You claimed your daily reward by raking",
            "You already claimed your daily reward",
        ),
    };
    let time = msg.timestamp.timestamp();
    let next_time = get_from_user(field, user_id, get_pool!(ctx)).await + delay;
    if next_time > time {
        return builder.content(format!("{method}, please try again <t:{next_time}:R>.",));
    }
    let mut exp = random_range(exp_range);
    let (passives, min, max, mult) =
        get_passives(user_id, msg.timestamp.timestamp(), get_pool!(ctx))
            .await
            .iter()
            .filter_map(|(passive_id, _)| {
                Passive::from_repr(*passive_id as usize).and_then(|p| {
                    let (min, max, mult, chance) = p.modifiers();
                    if random_bool(chance) {
                        Some((p, min, max, mult))
                    } else {
                        None
                    }
                })
            })
            .fold(
                (String::new(), 1., 1., 1.),
                |(names, a, b, c), (name, x, y, z)| {
                    (names + "\n- " + name.as_ref(), a + x, b + y, c + z)
                },
            );
    let mut leaves = (random_range((leaves_start * min).round()..(leaves_end * max).round()) * mult)
        .round() as i32;
    let mut embed = CreateEmbed::new()
        .title(format!("{remark} with `bare hands`.")) // change later
        .description(format!("`{exp:+} exp`\n`{leaves:+} Leaves`"))
        .color(POSITIVE);
    if !passives.is_empty() {
        embed = embed.field("Passives", passives, false)
    }
    if exp == leaves {
        exp += exp;
        leaves += leaves;
        let passive = if exp == 0 {
            Passive::LuckyZero
        } else if exp > 0 {
            Passive::Lucky
        } else {
            Passive::Unlucky
        };
        embed = embed.field(passive.as_ref(), passive.as_ref(), false);
        add_passive(user_id, time + passive.duration(), passive, get_pool!(ctx)).await;
    }
    update_raking(user_id, exp, leaves, field, time, get_pool!(ctx)).await;
    if let Some(item) = match random_range(0..10000) {
        q if q < 500 => Some(LeafHandful),   // 5%
        q if q < 600 => Some(LeafPile),      // 1%
        q if q < 625 => Some(LeafBucket),    // .25%
        q if q < 630 => Some(LeafBarrel),    // .05%
        q if q < 631 => Some(LeafTruckload), // .01%
        _ => None,
    } {
        embed = embed.field(
            "Bonus",
            format!(
                "You also found a `{}`!\n*It is now in your inventory.*",
                item.as_ref()
            ),
            true,
        );
        add_item(user_id, Item::from_base(item), get_pool!(ctx)).await;
    }
    builder.embed(embed)
}

async fn get_lb_string(ctx: &Context, server_ids: Vec<u64>) -> String {
    let mut top10 = String::new();
    for (id, exp) in get_lb(get_pool!(ctx), server_ids, Some(10)).await {
        top10 += &if let Ok(user) = UserId::new(id).to_user(&ctx).await {
            format!("1. `{:<24}{exp:>8} exp`\n", user.name)
        } else {
            format!("1. `{:<24}{exp:>8} exp`\n", "???")
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

fn handle_help(input: &str) -> CreateEmbed {
    match input {
        "rake" | "r" => CreateEmbed::new()
            .title("`rake` (alias: `r`)")
            .description("The basic command to rake and obtain Leaves.")
            .color(POSITIVE)
            .field("Details", "**exp**:\n`5`-`10`\n\
                **Leaves**:\n`(1 + 0.1 * <strength>)`-`(4 + 0.5 * <rake size> * <rake efficiency>)`\n\
                **cooldown**:\n`(30 + <rake size>)` seconds", false)
            .field("Drops", GIFT_DROPCHANCE, false),
        "riskyRake" | "rr" => CreateEmbed::new()
            .title("`riskyRake` (alias: `rr`)")
            .description("The risky version of raking to obtain or lose Leaves.")
            .color(POSITIVE)
            .field("Details", "**exp**:\n`-10`-`40`\n\
                **Leaves**:\n`(-6 + 0.2 * <strength>)`-`(16 + <rake size> * <rake efficiency>)`\n\
                **cooldown**:\n`(60 + 2 * <rake size>)` seconds", false)
            .field("Drops", GIFT_DROPCHANCE, false),
        _ => CreateEmbed::new()
            .title(format!("What's `{input}`?"))
            .description("I don't have that command, try `oi help` to see available commands.")
            .color(DANGER)
    }
}

fn handle_owner_help() -> CreateEmbed {
    CreateEmbed::new()
        .title("Gaia Commands")
        .description("Codename Gaia. Heed commands from her director.")
        .color(POSITIVE)
        .fields(vec![
            (
                "`give`",
                "Gives an item to a user.\n- `oi gaia give <user_id> <item_id>`",
                false,
            ),
            (
                "`apply`",
                "Applies a passive to a user.\n- `oi gaia apply <user_id> <passive_id> <duration>`",
                false,
            ),
            (
                "`bless`",
                "Provides exp and Leaves to a user.\n- `oi gaia bless <user_id> <exp> <leaves>`",
                false,
            ),
            ("`help`", "Displays this command.", false),
        ])
        .footer(CreateEmbedFooter::new("May your backyard be full of Leaves.").icon_url(ICON_URL))
}

async fn handle_owner_commands(
    ctx: &Context,
    user_id: i64,
    timestamp: i64,
    input: &str,
) -> CreateEmbed {
    if get_owner_id!(ctx) == &user_id {
        match input.split_once(" ") {
            Some((command, args)) => match command {
                "give" => match args.split_once(" ") {
                    Some((receiver_id, item_id)) => {
                        if let Ok(user_id) = receiver_id.parse()
                            && let Ok(item) = item_id.parse()
                            && let Ok(user) = UserId::new(user_id).to_user(&ctx).await
                            && let Some(base) = BaseItem::from_repr(item)
                        {
                            let embed = CreateEmbed::new()
                                .title("Item Granted")
                                .description(format!(
                                    "{} has received `{}`.",
                                    user.name,
                                    base.as_ref()
                                ))
                                .color(POSITIVE);
                            add_item(user_id as i64, Item::from_base(base), get_pool!(ctx)).await;
                            embed
                        } else {
                            CreateEmbed::new()
                                .title("Gaia is Confused by Your Demands")
                                .description("She does not recognize the user ID or item ID.")
                                .color(DANGER)
                        }
                    }
                    _ => CreateEmbed::new()
                        .title("Gaia is Confused by Your Demands")
                        .description("She does not sense an item ID.")
                        .color(POSITIVE),
                },
                "apply" => match args.splitn(3, " ").collect::<Vec<_>>()[..] {
                    [receiver_id, passive_id, seconds] => {
                        if let Ok(user_id) = receiver_id.parse()
                            && let Ok(passive) = passive_id.parse()
                            && let Ok(duration) = seconds.parse::<i64>()
                            && let Ok(user) = UserId::new(user_id).to_user(&ctx).await
                            && let Some(passive) = Passive::from_repr(passive)
                        {
                            let expires_at = timestamp + duration;
                            let embed = CreateEmbed::new()
                                .title("Passive Granted")
                                .description(format!(
                                    "Inflicted {} with `{}` until <t:{expires_at}:R>.",
                                    user.name,
                                    passive.as_ref()
                                ))
                                .color(POSITIVE);
                            add_passive(user_id as i64, expires_at, passive, get_pool!(ctx)).await;
                            embed
                        } else {
                            CreateEmbed::new()
                                .title("Gaia is Confused by Your Demands")
                                .description("She does not recognize the user/passive ID or time.")
                                .color(DANGER)
                        }
                    }
                    _ => CreateEmbed::new()
                        .title("Gaia is Confused by Your Demands")
                        .description("She does not sense user/passive ID or time.")
                        .color(DANGER),
                },
                "bless" => match args.splitn(3, " ").collect::<Vec<_>>()[..] {
                    [receiver_id, xp, amount] => {
                        if let Ok(user_id) = receiver_id.parse()
                            && let Ok(exp) = xp.parse()
                            && let Ok(leaves) = amount.parse()
                            && let Ok(user) = UserId::new(user_id).to_user(&ctx).await
                        {
                            update_raking(
                                user_id as i64,
                                exp,
                                leaves,
                                "last_raked",
                                timestamp,
                                get_pool!(ctx),
                            )
                            .await;
                            CreateEmbed::new()
                                .title("Wish Granted")
                                .description(format!(
                                    "{} has received `{exp} exp` and `{leaves} Leaves`.",
                                    user.name
                                ))
                                .color(POSITIVE)
                        } else {
                            CreateEmbed::new()
                                .title("Gaia is Confused by Your Demands")
                                .description("She does not recognize the user ID or exp/Leaves.")
                                .color(DANGER)
                        }
                    }
                    _ => CreateEmbed::new()
                        .title("Gaia is Confused by Your Demands")
                        .description("She does not sense user ID or exp/Leaves amount.")
                        .color(DANGER),
                },
                _ => CreateEmbed::new()
                    .title("Gaia is Confused by Your Command")
                    .description(format!("She does not recognize `{command}`."))
                    .color(DANGER),
            },
            None => match input {
                "help" => handle_owner_help(),
                _ => CreateEmbed::new()
                    .title("Gaia is Confused by Your Command")
                    .description(format!("She does not recognize `{input}`."))
                    .color(DANGER),
            },
        }
    } else {
        CreateEmbed::new()
            .title("Access Denied")
            .description("You do not have the permissions for this command.")
            .color(DANGER)
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
        let user_id = msg.author.id.get() as i64;
        let mut builder = CreateMessage::new();
        builder = match content.split_once(" ") {
            Some((command, input)) => match command {
                "help" => builder.embed(handle_help(input)),
                "say" | "say," => builder.embed(CreateEmbed::new()
                    .title("Question")
                    .description(input)
                    .color(POSITIVE)
                    .field("Answer", *RESPONSES.choice(input), true)),
                "speak" => builder.content(input).allowed_mentions(CreateAllowedMentions::new()),
                "info" => {
                    if let Ok(item) = BaseItem::from_str(input) {
                        let quantity = if item.equipable_limbs().is_empty() {
                            // can't have quality and modifier, default to 0
                            get_item(user_id, item as u32, 0, 0, get_pool!(ctx)).await
                        } else {
                            // TODO: change 0 when functionality is added to quality and modifier
                            get_item(user_id, item as u32, 0, 0, get_pool!(ctx)).await
                        };
                        if let Some(n) = quantity {
                            builder.embed(CreateEmbed::new()
                                .title(format!("You Own {n} of {input}"))
                                .description(item.description())
                                .color(POSITIVE))
                        } else {
                            builder.embed(CreateEmbed::new()
                                .title(format!("You don't own the item `{input}`."))
                                .description("Tough luck.")
                                .color(DANGER))
                        }
                    } else {
                        builder.embed(CreateEmbed::new()
                            .title(format!("`{input}` is not a valid item."))
                            .description("Did you capitalize the name?")
                            .field("Example", "`oi info T-Shirt`", true)
                            .color(DANGER))
                    }
                }
                "sell" => {
                    if let Some(item) = Item::from_str(input) {
                        let amount = get_item(user_id, item.base as u32, 0, 0, get_pool!(ctx)).await.unwrap();
                        builder.embed(CreateEmbed::new()
                            .title(format!("Are you sure that you want to sell {amount} of `{}`?", item.base.as_ref()))
                            .fields([
                                ("Original Price", format!("{} Leaves", item.base.buying_price()), true),
                                ("Selling Price", format!("{} Leaves", item.base.selling_price()), true)
                            ])
                            .color(WARNING))
                    } else {
                        builder.embed(CreateEmbed::new()
                            .title(format!("You don't own the item `{input}`."))
                            .description("Did you capitalize the item name?")
                            .color(DANGER))
                    }
                }
                // Bot owner exclusive command
                "gaia" => {
                    builder.embed(handle_owner_commands(&ctx, user_id, msg.timestamp.timestamp(), input).await)
                }
                _ => builder.embed(CreateEmbed::new()
                    .title(format!("What's `{command}`?"))
                    .description("I can't quite understand what you're saying, maybe try `oi help`?")
                    .color(DANGER))
            }
            None => match content {
                "help" => builder.embed(CreateEmbed::new()
                    .title("Commands")
                    .description("[Join our official server!](https://discord.gg/fwNnyndEM2)")
                    .color(POSITIVE)
                    .thumbnail(ICON_URL)
                    .fields(vec![
                        ("Raking", "`rake (r)`, `riskyRake (rr)`, `daily`, `rank`, `passives`, \
                            `leaderboard (lb)`, `shop`, `inventory (inv)`, `character (char)`, \
                            `equip`, `unequip`, `info`, `sell`, `arena (pvp)`", false),
                        ("Fun", "`say`", false),
                        ("Utility", "`ping`, `invite`", false),
                        ("Music", "`play`, `leave`", false),
                        ("Admin", "`speak`, `settings`", false),
                    ])
                    .footer(CreateEmbedFooter::new("yee haw").icon_url(ICON_URL))),
                "ping" => builder.embed(CreateEmbed::new()
                    .title("🏓 Pong!")
                    .color(POSITIVE)
                    .timestamp(Timestamp::now())),
                "invite" => builder.embed(CreateEmbed::new()
                    .title("Invite me to your server!")
                    .description("[Click this if you're an epic gamer or something idk]\
                        (https://discord.com/api/oauth2/authorize?client_id=767768980043333642&permissions=3435841&scope=bot)")
                    .color(POSITIVE)
                    .thumbnail("https://cdn.discordapp.com/avatars/767768980043333642/75c9a79a0aad1157fa8645061601961e.png")),
                "rake" | "r" => {
                    raking(&ctx, &msg, builder, RakeType::Normal).await
                }
                "riskyRake" | "rr" => {
                    raking(&ctx, &msg, builder, RakeType::Risky).await
                }
                "daily" => {
                    raking(&ctx, &msg, builder, RakeType::Daily).await
                }
                "inventory" | "inv" => builder.embed(CreateEmbed::new()
                    .title("Your Inventory")
                    .description(get_items(user_id, get_pool!(ctx))
                        .await
                        .into_iter()
                        .map(|(item, quantity)| item.as_owned(quantity))
                        .collect::<Vec<_>>().join("\n")
                        + &format!("\n\nYour Leaves: {}", get_from_user("leaves", user_id, get_pool!(ctx)).await)
                    )
                    .color(POSITIVE)),
                "passives" => {
                    let passives = get_passives(user_id, msg.timestamp.timestamp(), get_pool!(ctx)).await;
                    builder.embed(CreateEmbed::new()
                    .title(format!("You currently have {} passives", passives.len()))
                    .fields(passives
                        .into_iter().map(|(passive_id, time)| {
                            let p = Passive::from_repr(passive_id as usize).unwrap();
                            (format!("{} (expires <t:{time}:R>)", p.as_ref()), p.description(), false)
                        }))
                    .color(POSITIVE))
                }
                "shop" => {
                    let refresh_time = (msg.timestamp.timestamp() as u64 / 86400 + 1) * 86400;
                    let leaves = get_from_user("leaves", user_id, get_pool!(ctx)).await.to_string();
                    let (equipments, weapons, consumables) = get_shop(refresh_time);
                    builder.embed(CreateEmbed::new()
                        .title("Equipments for sale")
                        .description(equipments.shop_rep(1))
                        .field("Weapons on sale", weapons.shop_rep(5), false)
                        .field("Consumables on sale", consumables.shop_rep(7), false)
                        .field("Info", format!("Shop refreshes <t:{refresh_time}:R>.\nYour Leaves: {leaves}"), false)
                        .color(POSITIVE))
                }
                "leaderboard" | "lb" => {
                    let loading_msg = msg.channel_id.send_message(&ctx.http,
                        CreateMessage::new().content("<a:loading:822855468731990058> loading...")
                        ).await.unwrap_or_default();
                    let top10global = get_lb_string(&ctx, vec![]).await;
                    let mut embed = CreateEmbed::new()
                        .title("Global Leaderboard")
                        .description(top10global)
                        .color(POSITIVE);
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
                    .title(format!("What's `{content}`?"))
                    .description("I can't quite understand what you're saying, maybe try `oi help`?")
                    .color(DANGER))
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
    let owner_id = env::var("OWNER_ID")
        .expect("Expected owner ID in the environment")
        .parse::<i64>()
        .expect("Owner ID in the environment is in invalid format");
    try_create_tables(&pool).await;
    {
        let mut data = client.data.write().await;
        data.insert::<DbPool>(pool);
        data.insert::<OwnerId>(owner_id);
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
