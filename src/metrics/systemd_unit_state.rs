use std::fmt::Display;
use std::str::FromStr;

use anyhow::Context;
use hashbrown::HashMap;
use tokio::sync::RwLock;
use zbus::zvariant::OwnedObjectPath;
use zbus_systemd::systemd1::{ManagerProxy, UnitProxy};

/// All possible return values for the `ActiveState` property.
///
/// <https://www.freedesktop.org/software/systemd/man/latest/systemd.html#Units>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveState {
    /// Started, bound, plugged in, …, depending on the unit type.
    Active,
    /// Stopped, unbound, unplugged, …, depending on the unit type.
    Inactive,
    /// Similar to inactive, but the unit failed in some way (process returned error code on exit,
    /// crashed, an operation timed out, or after too many restarts).
    Failed,
    /// Changing from inactive to active.
    Activating,
    /// Changing from active to inactive.
    Deactivating,
    /// Unit is inactive and a maintenance operation is in progress.
    Maintenance,
    /// Unit is active and it is reloading its configuration.
    Reloading,
    /// Unit is active and a new mount is being activated in its namespace.
    Refreshing,
}

impl ActiveState {
    /// Returns a slice of all states as string.
    pub fn all_states() -> &'static [&'static str] {
        &[
            "active",
            "inactive",
            "failed",
            "activating",
            "deactivating",
            "maintenance",
            "reloading",
            "refreshing",
        ]
    }
}

impl FromStr for ActiveState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "failed" => Ok(Self::Failed),
            "activating" => Ok(Self::Activating),
            "deactivating" => Ok(Self::Deactivating),
            "maintenance" => Ok(Self::Maintenance),
            "reloading" => Ok(Self::Reloading),
            "refreshing" => Ok(Self::Refreshing),
            state => Err(anyhow::anyhow!("unknown state: {state}")),
        }
    }
}

impl Display for ActiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
            Self::Failed => write!(f, "failed"),
            Self::Activating => write!(f, "activating"),
            Self::Deactivating => write!(f, "deactivating"),
            Self::Maintenance => write!(f, "maintenance"),
            Self::Reloading => write!(f, "reloading"),
            Self::Refreshing => write!(f, "refreshing"),
        }
    }
}

/// Receive metrics from Systemd.
#[derive(Debug)]
pub struct SystemdUnitState<'proxy> {
    /// Cached connection to the D-Bus.
    connection: zbus::Connection,
    /// Cached manager.
    manager: zbus_systemd::systemd1::ManagerProxy<'proxy>,
    /// Key is a unit name, value is the path to the unitproxy.
    cached_units: RwLock<HashMap<String, OwnedObjectPath>>,
}

impl SystemdUnitState<'_> {
    /// Create a new metric query object. Keep this object in memory to cache
    /// relevant information.
    pub async fn new() -> anyhow::Result<Self> {
        let connection = zbus::connection::Builder::system()
            .context("creating d-bus connection builder")?
            .build()
            .await
            .context("connecting to system d-bus")?;

        let manager = ManagerProxy::new(&connection)
            .await
            .context("connecting to manager")?;

        Ok(Self {
            connection,
            manager,
            cached_units: RwLock::new(HashMap::new()),
        })
    }

    /// Retrieve the active state of the specified unit. Unit name must have the suffix (e.g.,
    /// `.service`).
    pub async fn active_state(&self, unit: &str) -> anyhow::Result<ActiveState> {
        let unit_path = {
            let reader_guard = self.cached_units.read().await;
            if reader_guard.contains_key(unit) {
                reader_guard
                    .get(unit)
                    .context("retrieving cached objectpath")?
                    .to_owned()
            } else {
                let unit_path = self
                    .manager
                    .get_unit(unit.to_owned())
                    .await
                    .with_context(|| format!("finding path to unit: {unit}"))?;
                drop(reader_guard);
                let mut writer = self.cached_units.write().await;
                writer.insert(unit.to_owned(), unit_path.clone());
                unit_path
            }
        };

        let unit = UnitProxy::new(&self.connection, &unit_path)
            .await
            .with_context(|| format!("creating unitproxy for {unit_path:?}"))?;
        let state = unit
            .active_state()
            .await
            .with_context(|| format!("retrieving activestate for {unit_path:?}"))?;

        let ret = state.parse().context("parsing activestate")?;
        Ok(ret)
    }
}
