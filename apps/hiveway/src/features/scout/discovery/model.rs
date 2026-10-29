use crate::features::common::model::Capability;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
pub struct PageQuery {
    pub q: Option<String>,
    pub runtime: Option<String>,
    pub creator_wallet: Option<String>,
    #[serde(default)]
    pub mine: bool,
    #[serde(default)]
    pub include_inactive: bool,
    pub limit: Option<u8>,
    pub cursor: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct CapabilityPage {
    pub items: Vec<Capability>,
    pub next_cursor: Option<String>,
}
