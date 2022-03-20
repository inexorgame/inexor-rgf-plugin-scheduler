use crate::reactive::NamedProperties;
use indradb::{Identifier, NamedProperty};
use serde_json::{json, Value};
use strum_macros::{AsRefStr, Display, IntoStaticStr};

#[allow(non_camel_case_types)]
#[derive(AsRefStr, IntoStaticStr, Display)]
pub enum ScheduledJobProperties {
    #[strum(serialize = "schedule")]
    SCHEDULE,
    #[strum(serialize = "trigger")]
    TRIGGER,
}

impl ScheduledJobProperties {
    pub fn default_value(&self) -> Value {
        match self {
            ScheduledJobProperties::SCHEDULE => json!(false),
            ScheduledJobProperties::TRIGGER => json!(0),
        }
    }
    pub fn properties() -> NamedProperties {
        vec![
            NamedProperty::from(ScheduledJobProperties::SCHEDULE),
            NamedProperty::from(ScheduledJobProperties::TRIGGER),
        ]
    }
}

impl From<ScheduledJobProperties> for NamedProperty {
    fn from(p: ScheduledJobProperties) -> Self {
        NamedProperty {
            name: Identifier::new(p.to_string()).unwrap(),
            value: p.default_value(),
        }
    }
}

impl From<ScheduledJobProperties> for String {
    fn from(p: ScheduledJobProperties) -> Self {
        p.to_string()
    }
}
