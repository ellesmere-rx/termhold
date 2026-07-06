//! Colony events in [`EVENTS`] — [`EventEffect::Choice`] (y/n) or [`EventEffect::Auto`].
//!
//! # How events are picked (`try_roll_event` in [`super::Game`])
//!
//! At the **end of each day** the game walks [`EVENTS`] **in array order**:
//!
//! 1. Skip if `day < min_day`.
//! 2. Skip if [`GameEvent::once`] and this id already fired.
//! 3. Skip if [`GameEvent::requires_building`] is set but not built yet.
//! 4. Skip if [`GameEvent::cooldown_days`] has not elapsed since the last answer.
//! 5. [`GameEvent::exact_day`] or `chance_percent` roll; fire once per day.
//! 6. Choice → pending; Auto → immediate effect + log.
//!
//! [`once`] and cooldown are recorded after a successful **Choice** answer or **Auto** effect (`Ok`).
//!
//! # Paying for "yes"
//!
//! If a choice costs resources and the colony cannot pay, the **no** outcome is applied
//! instead of blocking on `Err`.

use super::building::BuildingKind;
use super::{Balance, Colony};
use rand::RngExt;

/// Active event waiting for the player's y/n answer.
pub struct PendingEvent {
    /// Key into [`EVENTS`] / [`find_event`].
    pub event_id: &'static str,
}

/// What happens when an event fires.
pub enum EventEffect {
    Choice {
        prompt: &'static str,
        on_yes: fn(&mut Colony, &Balance) -> Result<&'static str, &'static str>,
        on_no: fn(&mut Colony, &Balance) -> Result<&'static str, &'static str>,
    },
    Auto(fn(&mut Colony, &Balance) -> Result<&'static str, &'static str>),
}

/// Random or scripted colony event.
pub struct GameEvent {
    pub id: &'static str,
    pub title: &'static str,
    pub min_day: usize,
    /// Per-day chance (`0` if only [`Self::exact_day`] should fire).
    pub chance_percent: u8,
    /// Guaranteed on this calendar day (checked before chance roll).
    pub exact_day: Option<usize>,
    pub once: bool,
    pub cooldown_days: Option<usize>,
    /// Roll only if at least one instance exists (`None` = no requirement).
    pub requires_building: Option<BuildingKind>,
    pub effect: EventEffect,
}

/// Lookup an event definition by [`GameEvent::id`] (stored in [`PendingEvent`]).
pub fn find_event(id: &str) -> Option<&'static GameEvent> {
    EVENTS.iter().find(|event| event.id == id)
}

impl GameEvent {
    /// Prompt for [`EventEffect::Choice`] events shown in the EVENT UI.
    pub fn choice_prompt(&self) -> Option<&'static str> {
        match self.effect {
            EventEffect::Choice { prompt, .. } => Some(prompt),
            EventEffect::Auto(_) => None,
        }
    }
}

pub const EVENTS: &[GameEvent] = &[
    // --- pressure (повторяющиеся) ---
    // wild_beasts — дикие звери у окраины
    // title: Каждую ночь следы кольцом вокруг колонии. Что-то ждёт за пределами света факелов.
    // prompt: Укрепить частокол дополнительным лесом? (y/n)
    // yes: Частокол укреплён. −8 дерева. | нет дерева: звери ограбили крайние склады. −6 еды
    // no: Провизия пропала с крайних складов. −6 еды
    GameEvent {
        id: "wild_beasts",
        title: "Tracks circle the colony each night. Something waits just beyond the torchlight.",
        min_day: 8,
        chance_percent: 8,
        once: false,
        exact_day: None,
        cooldown_days: Some(25),
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Reinforce the palisade with extra timber? (y/n)",
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
    },
    // blighted_barn — заражённый амбар
    // title: Из амбара тянет сладкой гнилью. Зерно в чёрной плесени.
    // prompt: Сжечь заражённые запасы немедленно? (y/n)
    // yes: Сожгли заражённое. −8 еды. | не хватает еды: как при отказе (плесень)
    // no: 30% −15 еды, иначе −5 еды
    GameEvent {
        id: "blighted_barn",
        title: "A sweet rot rises from the barn. The grain is speckled with black mold.",
        min_day: 8,
        chance_percent: 8,
        once: false,
        exact_day: None,
        cooldown_days: Some(35),
        requires_building: Some(BuildingKind::Barn),
        effect: EventEffect::Choice {
            prompt: "Burn the tainted stores immediately? (y/n)",
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
    },
    // sleepless_nights — бессонные ночи
    // title: Третью ночь половина лагеря не спит. Царапины в ставни, шаги за окнами — ближе не подходят.
    // prompt: Держать сторожевые костры всю ночь? (y/n)
    // yes: Костры до рассвета. −5 дерева, −2 еды. | не хватает: 20% −1 поселенец
    // no: 20% поселенец ушёл в темноту и не вернулся
    GameEvent {
        id: "sleepless_nights",
        title: "For three nights half the camp has not slept. Scratches at the shutters, footsteps circling outside — never close enough to see.",
        min_day: 10,
        chance_percent: 7,
        once: false,
        exact_day: None,
        cooldown_days: Some(35),
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Keep watch-fires burning through the dark hours? (y/n)",
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
    },
    // --- resources & price (ресурсы и цена) ---
    // dry_roots — сухие корни
    // title: Лесорубы нашли полые гиганты среди корней — сухие как трух, тёплые на ощупь.
    // prompt: Срубить и вывезти древесину? (y/n)
    // yes: +25 дерева, −5 еды на сушильные костры
    // no: Полые деревья оставили стоять
    GameEvent {
        id: "dry_roots",
        title: "Lumberhands found hollow giants among the roots — dry as tinder, warm to the touch.",
        min_day: 12,
        chance_percent: 7,
        once: false,
        exact_day: None,
        cooldown_days: Some(25),
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Fell them and haul the timber back? (y/n)",
            on_yes: |c, _| {
                c.wood += 25;
                c.food = c.food.saturating_sub(5);
                Ok("Haulers returned laden. +25 wood, -5 food burned for drying fires.")
            },
            on_no: |_, _| Ok("The hollow trees were left standing."),
        },
    },
    // hollow_stone — полый камень в каменоломне
    // title: Стена каменолома звенит пустотой. Бригадир клянётся — в темноте за породой что-то шевелилось.
    // prompt: Подорвать глубже и взять что можно? (y/n)
    // yes: +6 камня, 15% обвал — −1 поселенец
    // no: Подозрительную стену засыпали
    GameEvent {
        id: "hollow_stone",
        title: "The quarry face rings hollow. The foreman swears something moved in the dark behind the rock.",
        min_day: 15,
        chance_percent: 6,
        once: false,
        exact_day: None,
        cooldown_days: Some(30),
        requires_building: Some(BuildingKind::StoneQuarry),
        effect: EventEffect::Choice {
            prompt: "Go deeper and take what you can? (y/n)",
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
    },
    // ashen_clearing — пепельная прогалина
    // title: Поляна обугленных стволов, не тронутых пламенем. Пепел не шевелится на ветру.
    // prompt: Рубить лес на мёртвой поляне? (y/n)
    // yes: +15 дерева, пепел испортил пайки в мешках. −3 еды
    // no: Обошли пепельную поляну
    GameEvent {
        id: "ashen_clearing",
        title: "A clearing of char-black trunks stands untouched by flame. The ash does not stir in the wind.",
        min_day: 14,
        chance_percent: 6,
        once: false,
        exact_day: None,
        cooldown_days: Some(30),
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Cut timber from the dead stand? (y/n)",
            on_yes: |c, _| {
                c.wood += 15;
                c.food = c.food.saturating_sub(3);
                Ok("+15 wood. Ash spoiled rations in the sacks. -3 food.")
            },
            on_no: |_, _| Ok("The ashen clearing was avoided."),
        },
    },
    // --- colony (свои люди) ---
    // wall_symbols — знаки на стене
    // title: За ночь на стене хижины появились странные метки. Никто не признаётся, что вырезал.
    // prompt: Сорвать доски и перестроить стену? (y/n)
    // yes: Доски сожгли. −3 дерева. | нет дерева: 15% смерть у знаков
    // no: 15% поселенец перестал дышать у знаков на рассвете
    GameEvent {
        id: "wall_symbols",
        title: "Strange marks appeared overnight on fresh-cut boards. No settler admits to carving them.",
        min_day: 12,
        chance_percent: 5,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Tear down the boards and rebuild the wall? (y/n)",
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
                    Ok(
                        "A settler was found staring at the marks at dawn, then simply stopped breathing.",
                    )
                } else {
                    Ok("The marks remain. The nights grow colder.")
                }
            },
        },
    },
    // stump_tithe — дань у пня
    // title: У древнего пня на опушке сочится чёрная смола. Все видят один сон — корни сжимают лодыжки.
    // prompt: Сжечь запасы у корней, пока лес не возьмёт плату плотью? (y/n)
    // yes: −6 еды, смола перестала течь. | не хватает еды: 35% лес забирает поселенца
    // no: Пень остался голодным
    GameEvent {
        id: "stump_tithe",
        title: "Black resin weeps from an ancient stump at the treeline. Settlers wake with the same dream of roots closing around their ankles.",
        min_day: 16,
        chance_percent: 4,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Burn stores at its roots before the forest takes payment in flesh? (y/n)",
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
    },
    // --- forest & darkness (лес и тьма) ---
    // blackforest — чёрный лес
    // title: Лесорубы нашли участок глубже, чем на картах экспедиции. Деревья огромные… и безмолвные.
    // prompt: Отправить лесоруба исследовать лес? (y/n)
    // yes: +20 дерева, 20% лесоруб не вернулся
    // no: Лесоруб не тронул безмолвные деревья
    GameEvent {
        id: "blackforest",
        title: "Lumberhands report timber deeper than your maps showed. The trees are enormous... and strangely silent.",
        min_day: 10,
        chance_percent: 5,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Send a crew to cut there? (y/n)",
            on_yes: |c, balance| {
                c.wood += 20;
                if c.population > 0 && rand::rng().random_range(0..100) < 20 {
                    c.population -= 1;
                    c.clamp_workers(balance);
                    Ok("+20 wood from the black stand. The crewman never returned.")
                } else {
                    Ok("+20 wood from the black stand.")
                }
            },
            on_no: |_, _| Ok("The silent trees were left uncut."),
        },
    },
    // treeline_whisper — шёпот на опушке
    // title: После темноты с опушки доносится шёпот. Слов на языке колонии нет.
    // prompt: Послать кого-нибудь к краю узнать, кто говорит? (y/n)
    // yes: 25% разведчик не вернулся в свет факелов | иначе вернулся оглушённый
    // no: Караула не было. −3 еды (ночные пропажи)
    GameEvent {
        id: "treeline_whisper",
        title: "Whispers drift from the treeline after dark. Words in no tongue your settlers know.",
        min_day: 18,
        chance_percent: 4,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Send someone to the edge to learn what speaks? (y/n)",
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
    },
    // moving_roots — движущиеся корни
    // title: Фермеры клянутся: корни под полем сдвинулись между закатом и рассветом.
    // prompt: Вскопать и посмотреть, что под ними? (y/n)
    // yes: +10 еды, 20% одного закопало
    // no: Поле не трогали. Урожай провалился клочьями. −4 еды
    GameEvent {
        id: "moving_roots",
        title: "Farmhands swear the roots beneath the field shifted between dusk and dawn.",
        min_day: 20,
        chance_percent: 5,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: Some(BuildingKind::Farm),
        effect: EventEffect::Choice {
            prompt: "Dig them up and see what lies underneath? (y/n)",
            on_yes: |c, balance| {
                c.food += 10;
                if c.population > 0 && rand::rng().random_range(0..100) < 20 {
                    c.population -= 1;
                    c.clamp_workers(balance);
                    Ok("+10 food from buried stores. One digger was pulled under.")
                } else {
                    Ok("+10 food from buried stores in rotted cloth.")
                }
            },
            on_no: |c, _| {
                c.food = c.food.saturating_sub(4);
                Ok("The field was left alone. The crop failed in patches. -4 food.")
            },
        },
    },
    // --- ruins & ancient (руины и древнее) ---
    // bone_pit — яма с костями (расчистка лагеря)
    // title: При расчистке площадки под лагерь — яма костей. Слишком длинные, слишком много суставов.
    // prompt: Взять кремень и обтёсанный камень, остальное закопать? (y/n)
    // yes: +8 камня, 25% что-то в яме не осталось закопанным
    // no: Яму засыпали и пометили предупреждениями
    GameEvent {
        id: "bone_pit",
        title: "Surveying the camp site, your diggers open a pit of bones — too long, too many joints.",
        min_day: 22,
        chance_percent: 4,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Take the flint and dressed stone, then bury the rest? (y/n)",
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
    },
    // quarry_ruins — руины за каменоломом
    // title: За обвалом — стена из чёрного камня без швов, старше Империи.
    // prompt: Разобрать на строительный камень? (y/n)
    // yes: +12 камня, 15% чёрная стена забрала жизнь при обрушении
    // no: Проход засыпали щебнем и мелом
    GameEvent {
        id: "quarry_ruins",
        title: "Behind a quarry collapse, a wall of black stone stands seamless — older than the Empire, older than any map you carried.",
        min_day: 25,
        chance_percent: 4,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Break it apart for building stone? (y/n)",
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
    },
    // sealed_hatch — люк при рытье фундамента (не под «старой» хижиной)
    // title: Копая фундамент под новую хижину, наткнулись на железный люк — глубже, чем должна быть почва.
    // prompt: Раскопать и вскрыть? (y/n)
    // yes: 50% имперские запасы | 50% кто-то утащен
    // no: Засыпали и сдвинули постройку
    GameEvent {
        id: "sealed_hatch",
        title: "Digging a hut foundation, your settlers strike iron — a hatch sealed with wax and rust, buried far too deep for a fresh camp.",
        min_day: 30,
        chance_percent: 3,
        once: true,
        exact_day: None,
        cooldown_days: None,
        requires_building: None,
        effect: EventEffect::Choice {
            prompt: "Excavate and pry it open? (y/n)",
            on_yes: |c, balance| {
                if rand::rng().random_range(0..100) < 50 {
                    c.food += 20;
                    Ok("Sealed grain behind the hatch — imperial wax, still dry. +20 food.")
                } else if c.population > 0 {
                    c.population -= 1;
                    c.clamp_workers(balance);
                    Ok("Something below the hatch dragged them under.")
                } else {
                    Ok("The hatch slammed shut on empty air.")
                }
            },
            on_no: |_, _| Ok("The hatch was buried again. The hut went up on another spot."),
        },
    },
    // ash_fall — пепельный дождь (auto) — мир угасает
    // title: Серый пепел целый день. Ни одного костра на горизонте.
    // auto: −3 еды
    GameEvent {
        id: "ash_fall",
        title: "Grey ash falls all day. No fire burns on the horizon to raise it. Rations taste of soot.",
        min_day: 15,
        chance_percent: 6,
        exact_day: None,
        once: false,
        cooldown_days: Some(20),
        requires_building: None,
        effect: EventEffect::Auto(|c, _| {
            c.food = c.food.saturating_sub(3);
            Ok("Ash spoiled open stores. The fading reaches even clear sky. -3 food.")
        }),
    },
];
