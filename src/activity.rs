use rand::seq::IndexedRandom;
use twilight_model::gateway::presence::{Activity, ActivityType, MinimalActivity};

struct ActivityData<'a> {
    name: &'a str,
    kind: ActivityType,
}

static ACTIVITIES: &[ActivityData] = &[
    ActivityData {
        name: "the sky fall",
        kind: ActivityType::Watching,
    },
    ActivityData {
        name: "lofi hip-hop 🎶",
        kind: ActivityType::Listening,
    },
    ActivityData {
        name: "with your heart ❤️",
        kind: ActivityType::Playing,
    },
    ActivityData {
        name: "and learning 📝",
        kind: ActivityType::Watching,
    },
];

pub fn get_random_activity() -> Activity {
    let activity = ACTIVITIES.choose(&mut rand::rng()).unwrap();

    let minimal_activity = MinimalActivity {
        name: activity.name.to_string(),
        kind: activity.kind,
        url: None,
    };
    Activity::from(minimal_activity)
}

static QOUTES: &[&str] = &[
    "\"Gravity is a harness. I have harnessed the harness.\" - Sigma (Overwatch)",
    "\"...time, Dr. Freeman? Is it really that time again? It seems as if you only just arrived.\" - The G-Man (Half-Life 2)",
    "\"Some believe the fate of our worlds is inflexible. My employers disagree. They authorize me to... nudge things in a particular direction from time to time.\" - The G-Man (Half-Life Alyx)",
    "\"There may be honor among thieves, but there's none in politicians.\" - Lawrence of Arabia",
    "\"Look at you. Sailing through the air majestically. Like an eagle. Piloting a blimp.\" - GLaDOS (Portal 2)",
    "\"I'm Spartacus!\" - Antoninus",
    "\"Sextus, you ask how to fight an idea. Well I'll tell you how... with another idea!\" - Messala (Ben-Hur)"
    // ... add more
];

pub fn get_random_qoute() -> String {
    let qoute = QOUTES.choose(&mut rand::rng()).unwrap();
    qoute.to_string()
}
