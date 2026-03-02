pub mod dashboard;
pub mod events;
pub mod platforms;
pub mod publishers;
pub mod resolve;
pub mod sessions;

/// The tabs available in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Sessions,
    Events,
    Publishers,
    Platforms,
    Resolve,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Dashboard,
        Tab::Sessions,
        Tab::Events,
        Tab::Publishers,
        Tab::Platforms,
        Tab::Resolve,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Sessions => "Sessions",
            Tab::Events => "Events",
            Tab::Publishers => "Publishers",
            Tab::Platforms => "Platforms",
            Tab::Resolve => "Resolve",
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Tab::ALL.get(i).copied().unwrap_or(Tab::Dashboard)
    }

    pub fn next(self) -> Self {
        Tab::from_index((self.index() + 1) % Tab::ALL.len())
    }

    pub fn prev(self) -> Self {
        let i = self.index();
        Tab::from_index(if i == 0 { Tab::ALL.len() - 1 } else { i - 1 })
    }
}
