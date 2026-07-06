//! Random yes/no events loaded from [`EVENTS`].
//!
//! # How events are picked (`try_roll_event` in [`super::Game`])
//!
//! At the **end of each day** the game walks [`EVENTS`] **in array order**:
//!
//! 1. Skip if `day < min_day`.
//! 2. Skip if [`YesNoEvent::once`] and this id already fired.
//! 3. Skip if [`YesNoEvent::cooldown_days`] has not elapsed since the last answer.
//! 4. Roll `0..100`; on success set pending and **stop** (one event per day).
//!
//! [`once`] and cooldown are recorded in [`super::Game`] only after a successful answer
//! (`Ok` from `on_yes` / `on_no`), not when the event is merely rolled.
//!
//! # Paying for "yes"
//!
//! If a choice costs resources and the colony cannot pay, the **no** outcome is applied
//! instead of blocking on `Err`.

use super::{Balance, Colony};
use rand::RngExt;

/// Active event waiting for the player's y/n answer.
pub struct PendingEvent {
    /// Key into [`EVENTS`] / [`find_event`].
    pub event_id: &'static str,
}

/// Definition of one yes/no random event.
pub struct YesNoEvent {
    pub id: &'static str,
    /// Shown in the log when the event fires and as the EVENT screen heading.
    pub title: &'static str,
    /// Shown under EVENT; should end with `(y/n)`.
    pub prompt: &'static str,
    /// First day this event can be rolled (inclusive).
    pub min_day: usize,
    /// Per-day trigger chance after `min_day` (`0..100` roll `<` this value).
    pub chance_percent: u8,
    /// At most one successful trigger per playthrough.
    pub once: bool,
    /// Days before this event can roll again after a successful answer (`None` = no cooldown).
    pub cooldown_days: Option<usize>,
    /// Apply yes choice; `Ok` = log line, `Err` = blocked (event stays pending).
    pub on_yes: fn(&mut Colony, &Balance) -> Result<&'static str, &'static str>,
    /// Apply no choice; same return convention as `on_yes`.
    pub on_no: fn(&mut Colony, &Balance) -> Result<&'static str, &'static str>,
}

/// Lookup an event definition by [`YesNoEvent::id`] (stored in [`PendingEvent`]).
pub fn find_event(id: &str) -> Option<&'static YesNoEvent> {
    EVENTS.iter().find(|event| event.id == id)
}

pub const EVENTS: &[YesNoEvent] = &[
    // --- pressure (повторяющиеся) ---
    // wild_beasts — дикие звери у окраины
    // title: Каждую ночь следы кольцом вокруг колонии. Что-то ждёт за пределами света факелов.
    // prompt: Укрепить частокол дополнительным лесом? (y/n)
    // yes: Частокол укреплён. −8 дерева. | нет дерева: звери ограбили крайние склады. −6 еды
    // no: Провизия пропала с крайних складов. −6 еды
    YesNoEvent {
        id: "wild_beasts",
        title: "Tracks circle the colony each night. Something waits just beyond the torchlight.",
        prompt: "Reinforce the palisade with extra timber? (y/n)",
        min_day: 8,
        chance_percent: 8,
        once: false,
        cooldown_days: Some(25),
        on_yes: |c, _| {
            if c.wood < 8 {
                c.food = c.food.saturating_sub(6);
                return Ok(
                    "Too little timber — the palisade stayed weak. Beasts raided the edge stores. -6 food.",
                );
            }
            c.wood -= 8;
            Ok("The palisade is strengthened. -8 wood.")
        },
        on_no: |c, _| {
            c.food = c.food.saturating_sub(6);
            Ok("Provisions vanish from the edge stores. -6 food.")
        },
    },
    // blighted_barn — заражённый амбар
    // title: Из амбара тянет сладкой гнилью. Зерно в чёрной плесени.
    // prompt: Сжечь заражённые запасы немедленно? (y/n)
    // yes: Сожгли заражённое. −8 еды. | не хватает еды: как при отказе (плесень)
    // no: 30% −15 еды, иначе −5 еды
    YesNoEvent {
        id: "blighted_barn",
        title: "A sweet rot rises from the barn. The grain is speckled with black mold.",
        prompt: "Burn the tainted stores immediately? (y/n)",
        min_day: 8,
        chance_percent: 8,
        once: false,
        cooldown_days: Some(35),
        on_yes: |c, _| {
            if c.food < 8 {
                if rand::rng().random_range(0..100) < 30 {
                    c.food = c.food.saturating_sub(15);
                    return Ok("The mold spread overnight. -15 food.");
                }
                c.food = c.food.saturating_sub(5);
                return Ok("Only a little spoiled. -5 food — it could have been worse.");
            }
            c.food -= 8;
            Ok("You burned the infected stores. -8 food.")
        },
        on_no: |c, _| {
            if rand::rng().random_range(0..100) < 30 {
                c.food = c.food.saturating_sub(15);
                Ok("The mold spread overnight. -15 food.")
            } else {
                c.food = c.food.saturating_sub(5);
                Ok("Only a little spoiled. -5 food — it could have been worse.")
            }
        },
    },
    // sleepless_nights — бессонные ночи
    // title: Третью ночь половина колонии не спит. Говорят о царапанье под половицами.
    // prompt: Держать сторожевые костры всю ночь? (y/n)
    // yes: Костры до рассвета. −5 дерева, −2 еды. | не хватает: 20% −1 поселенец
    // no: 20% поселенец ушёл в темноту и не вернулся
    YesNoEvent {
        id: "sleepless_nights",
        title: "For three nights half the colony has not slept. They speak of scratching under the floorboards.",
        prompt: "Keep watch-fires burning through the dark hours? (y/n)",
        min_day: 10,
        chance_percent: 7,
        once: false,
        cooldown_days: Some(35),
        on_yes: |c, balance| {
            if c.wood < 5 || c.food < 2 {
                if c.population > 0 && rand::rng().random_range(0..100) < 20 {
                    c.population -= 1;
                    c.clamp_workers(balance);
                    return Ok("A settler walked into the dark and did not return.");
                }
                return Ok("The colony endured the nights, barely.");
            }
            c.wood -= 5;
            c.food -= 2;
            Ok("Watch-fires burn until dawn. -5 wood, -2 food.")
        },
        on_no: |c, balance| {
            if c.population > 0 && rand::rng().random_range(0..100) < 20 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("A settler walked into the dark and did not return.")
            } else {
                Ok("The colony endured the nights, barely.")
            }
        },
    },
    // --- resources & price (ресурсы и цена) ---
    // dry_roots — сухие корни
    // title: Лесорубы нашли полые гиганты среди корней — сухие как трух, тёплые на ощупь.
    // prompt: Срубить и вывезти древесину? (y/n)
    // yes: +25 дерева, −5 еды на сушильные костры
    // no: Полые деревья оставили стоять
    YesNoEvent {
        id: "dry_roots",
        title: "Lumberhands found hollow giants among the roots — dry as tinder, warm to the touch.",
        prompt: "Fell them and haul the timber back? (y/n)",
        min_day: 12,
        chance_percent: 7,
        once: false,
        cooldown_days: Some(25),
        on_yes: |c, _| {
            c.wood += 25;
            c.food = c.food.saturating_sub(5);
            Ok("Haulers returned laden. +25 wood, -5 food burned for drying fires.")
        },
        on_no: |_, _| Ok("The hollow trees were left standing."),
    },
    // hollow_stone — полый камень в каменоломне
    // title: Стена каменолома звенит пустотой. Бригадир клянётся — в темноте за породой что-то шевелилось.
    // prompt: Подорвать глубже и взять что можно? (y/n)
    // yes: +6 камня, 15% обвал — −1 поселенец
    // no: Подозрительную стену засыпали
    YesNoEvent {
        id: "hollow_stone",
        title: "The quarry face rings hollow. The foreman swears something moved in the dark behind the rock.",
        prompt: "Go deeper and take what you can? (y/n)",
        min_day: 15,
        chance_percent: 6,
        once: false,
        cooldown_days: Some(30),
        on_yes: |c, balance| {
            c.stone += 6;
            if c.population > 0 && rand::rng().random_range(0..100) < 15 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("+6 stone. A collapse buried one worker.")
            } else {
                Ok("+6 stone from the hollow vein.")
            }
        },
        on_no: |_, _| Ok("The quarry sealed the suspicious face."),
    },
    // ashen_clearing — пепельная прогалина
    // title: Поляна обугленных стволов, не тронутых пламенем. Пепел не шевелится на ветру.
    // prompt: Рубить лес на мёртвой поляне? (y/n)
    // yes: +15 дерева, пепел испортил пайки в мешках. −3 еды
    // no: Обошли пепельную поляну
    YesNoEvent {
        id: "ashen_clearing",
        title: "A clearing of char-black trunks stands untouched by flame. The ash does not stir in the wind.",
        prompt: "Cut timber from the dead stand? (y/n)",
        min_day: 14,
        chance_percent: 6,
        once: false,
        cooldown_days: Some(30),
        on_yes: |c, _| {
            c.wood += 15;
            c.food = c.food.saturating_sub(3);
            Ok("+15 wood. Ash spoiled rations in the sacks. -3 food.")
        },
        on_no: |_, _| Ok("The ashen clearing was avoided."),
    },
    // --- colony (свои люди) ---
    // wall_symbols — знаки на стене
    // title: За ночь на стене хижины появились странные метки. Никто не признаётся, что вырезал.
    // prompt: Сорвать доски и перестроить стену? (y/n)
    // yes: Доски сожгли. −3 дерева. | нет дерева: 15% смерть у знаков
    // no: 15% поселенец перестал дышать у знаков на рассвете
    YesNoEvent {
        id: "wall_symbols",
        title: "Strange marks appeared on a hut wall overnight. No settler admits to carving them.",
        prompt: "Tear down the boards and rebuild the wall? (y/n)",
        min_day: 12,
        chance_percent: 5,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            if c.wood < 3 {
                if c.population > 0 && rand::rng().random_range(0..100) < 15 {
                    c.population -= 1;
                    c.clamp_workers(balance);
                    return Ok(
                        "A settler was found staring at the marks at dawn, then simply stopped breathing.",
                    );
                }
                return Ok("The marks remain. The nights grow colder.");
            }
            c.wood -= 3;
            Ok("The marked boards were burned. -3 wood.")
        },
        on_no: |c, balance| {
            if c.population > 0 && rand::rng().random_range(0..100) < 15 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("A settler was found staring at the marks at dawn, then simply stopped breathing.")
            } else {
                Ok("The marks remain. The nights grow colder.")
            }
        },
    },
    // stump_tithe — дань у пня
    // title: У древнего пня на опушке сочится чёрная смола. Все видят один сон — корни сжимают лодыжки.
    // prompt: Сжечь запасы у корней, пока лес не возьмёт плату плотью? (y/n)
    // yes: −6 еды, смола перестала течь. | не хватает еды: 35% лес забирает поселенца
    // no: Пень остался голодным
    YesNoEvent {
        id: "stump_tithe",
        title: "Black resin weeps from an ancient stump at the treeline. Settlers wake with the same dream of roots closing around their ankles.",
        prompt: "Burn stores at its roots before the forest takes payment in flesh? (y/n)",
        min_day: 16,
        chance_percent: 4,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            if c.food < 6 {
                if c.population > 0 && rand::rng().random_range(0..100) < 35 {
                    c.population -= 1;
                    c.clamp_workers(balance);
                    return Ok(
                        "Without a tithe, the forest took flesh. A settler was found at the treeline.",
                    );
                }
                return Ok("The stump was left hungry.");
            }
            c.food -= 6;
            Ok("Food burned at the stump until the resin stopped flowing. -6 food.")
        },
        on_no: |_, _| Ok("The stump was left hungry."),
    },
    // --- forest & darkness (лес и тьма) ---
    // blackforest — чёрный лес
    // title: Лесорубы нашли нетронутый участок леса. Деревья огромные… и странно безмолвные.
    // prompt: Отправить лесоруба исследовать лес? (y/n)
    // yes: +20 дерева, 20% лесоруб не вернулся
    // no: Лесоруб не тронул безмолвные деревья
    YesNoEvent {
        id: "blackforest",
        title: "Lumberjack report finding an untouched part of the forest. The trees are enormous... and strangely silent.",
        prompt: "Should the lumberjack explore the forest? (y/n)",
        min_day: 10,
        chance_percent: 5,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            c.wood += 20;
            if c.population > 0 && rand::rng().random_range(0..100) < 20 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("+20 wood from the black forest. The lumberjack never returned.")
            } else {
                Ok("+20 wood from the black forest.")
            }
        },
        on_no: |_, _| Ok("The lumberjack left the silent trees untouched."),
    },
    // treeline_whisper — шёпот на опушке
    // title: После темноты с опушки доносится шёпот. Слов на языке колонии нет.
    // prompt: Послать кого-нибудь к краю узнать, кто говорит? (y/n)
    // yes: 25% разведчик не вернулся в свет факелов | иначе вернулся оглушённый
    // no: Караула не было. −3 еды (ночные пропажи)
    YesNoEvent {
        id: "treeline_whisper",
        title: "Whispers drift from the treeline after dark. Words in no tongue the colony knows.",
        prompt: "Send someone to the edge to learn what speaks? (y/n)",
        min_day: 18,
        chance_percent: 4,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            if c.population > 0 && rand::rng().random_range(0..100) < 25 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("The scout never crossed back into the torchlight.")
            } else {
                Ok("They returned deaf to the whispers, unable to say what they saw.")
            }
        },
        on_no: |c, _| {
            c.food = c.food.saturating_sub(3);
            Ok("No watch was posted. -3 food lost to night thefts and dread.")
        },
    },
    // moving_roots — движущиеся корни
    // title: Фермеры клянутся: корни под полем сдвинулись между закатом и рассветом.
    // prompt: Вскопать и посмотреть, что под ними? (y/n)
    // yes: +10 еды, 20% одного закопало
    // no: Поле не трогали. Урожай провалился клочьями. −4 еды
    YesNoEvent {
        id: "moving_roots",
        title: "Farmhands swear the roots beneath the field shifted between dusk and dawn.",
        prompt: "Dig them up and see what lies underneath? (y/n)",
        min_day: 20,
        chance_percent: 5,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            c.food += 10;
            if c.population > 0 && rand::rng().random_range(0..100) < 20 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("+10 food from buried stores. One digger was pulled under.")
            } else {
                Ok("+10 food from caches wrapped in rotted cloth.")
            }
        },
        on_no: |c, _| {
            c.food = c.food.saturating_sub(4);
            Ok("The field was left alone. The crop failed in patches. -4 food.")
        },
    },
    // --- ruins & ancient (руины и древнее) ---
    // bone_pit — яма с костями
    // title: При расчистке завала открылась яма костей — слишком длинные, слишком много суставов.
    // prompt: Взять кремень и обтёсанный камень, остальное закопать? (y/n)
    // yes: +8 камня, 25% что-то в яме не осталось закопанным
    // no: Яму засыпали и пометили предупреждениями
    YesNoEvent {
        id: "bone_pit",
        title: "Clearing rubble uncovered a pit of bones — too long, too many joints.",
        prompt: "Take the flint and dressed stone, then bury the rest? (y/n)",
        min_day: 22,
        chance_percent: 4,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            c.stone += 8;
            if c.population > 0 && rand::rng().random_range(0..100) < 25 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("+8 stone. Something in the pit did not stay buried.")
            } else {
                Ok("+8 stone from the pit, sealed with cairn stones.")
            }
        },
        on_no: |_, _| Ok("The pit was filled and marked with warnings."),
    },
    // quarry_ruins — руины за каменоломом
    // title: За обвалом в каменоломне — стена из чёрного камня без швов, старше Termhold.
    // prompt: Разобрать на строительный камень? (y/n)
    // yes: +12 камня, 15% чёрная стена забрала жизнь при обрушении
    // no: Проход засыпали щебнем и мелом
    YesNoEvent {
        id: "quarry_ruins",
        title: "Behind a quarry collapse, a wall of black stone stands seamless, older than Termhold.",
        prompt: "Break it apart for building stone? (y/n)",
        min_day: 25,
        chance_percent: 4,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            c.stone += 12;
            if c.population > 0 && rand::rng().random_range(0..100) < 15 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("+12 stone from the ruin. The black wall took a life as it fell.")
            } else {
                Ok("+12 stone quarried from the ancient wall.")
            }
        },
        on_no: |_, _| Ok("The breach was sealed with rubble and chalk marks."),
    },
    // sealed_hatch — запечатанный люк
    // title: Под половицами старой хижины — железный люк, залитый воском и заржавевшими гвоздями.
    // prompt: Вскрыть? (y/n)
    // yes: 50% +20 еды в закупоренном зерне | 50% кто-то утащен в темноту
    // no: Люк прибили заново и закрыли новыми досками
    YesNoEvent {
        id: "sealed_hatch",
        title: "Beneath an old hut's floorboards lies an iron hatch sealed with wax and rusted nails.",
        prompt: "Pry it open? (y/n)",
        min_day: 30,
        chance_percent: 3,
        once: true,
        cooldown_days: None,
        on_yes: |c, balance| {
            if rand::rng().random_range(0..100) < 50 {
                c.food += 20;
                Ok("Sealed grain behind the hatch — still dry. +20 food.")
            } else if c.population > 0 {
                c.population -= 1;
                c.clamp_workers(balance);
                Ok("Something in the dark dragged them under.")
            } else {
                Ok("The hatch slammed shut on empty air.")
            }
        },
        on_no: |_, _| Ok("The hatch was re-nailed and covered with fresh boards."),
    },
];
