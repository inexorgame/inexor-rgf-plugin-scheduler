use crate::reactive::NamedProperties;
use indradb::{Identifier, NamedProperty};
use serde_json::{json, Value};
use strum_macros::{AsRefStr, Display, IntoStaticStr};

#[allow(non_camel_case_types)]
#[derive(AsRefStr, IntoStaticStr, Display)]
pub enum TimerProperties {
    #[strum(serialize = "duration")]
    DURATION,
    #[strum(serialize = "trigger")]
    TRIGGER,
}

impl TimerProperties {
    pub fn default_value(&self) -> Value {
        match self {
            TimerProperties::DURATION => json!(1000),
            TimerProperties::TRIGGER => json!(0),
        }
    }
    pub fn properties() -> NamedProperties {
        vec![NamedProperty::from(TimerProperties::DURATION), NamedProperty::from(TimerProperties::TRIGGER)]
    }
}

impl From<TimerProperties> for NamedProperty {
    fn from(p: TimerProperties) -> Self {
        NamedProperty {
            name: Identifier::new(p.to_string()).unwrap(),
            value: p.default_value(),
        }
    }
}

impl From<TimerProperties> for String {
    fn from(p: TimerProperties) -> Self {
        p.to_string()
    }
}
