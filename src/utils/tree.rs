use std::{cmp::Ordering, str::FromStr};

use anyhow::anyhow;

#[derive(Clone, Copy, Debug)]
pub enum TreeDepth {
    All,
    Depth(usize),
}

impl FromStr for TreeDepth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.eq_ignore_ascii_case("all") {
            Ok(Self::All)
        } else if let Ok(n) = s.parse::<usize>() {
            Ok(Self::Depth(n))
        } else {
            Err(anyhow!("Invalid value for TreeDepth: {}", s))
        }
    }
}

impl PartialEq<usize> for TreeDepth {
    fn eq(&self, other: &usize) -> bool {
        match self {
            Self::All => false,
            Self::Depth(n) => n == other,
        }
    }
}

impl PartialOrd<usize> for TreeDepth {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        match self {
            Self::All => Some(Ordering::Greater),
            Self::Depth(n) => n.partial_cmp(other),
        }
    }
}

impl PartialEq<TreeDepth> for usize {
    fn eq(&self, other: &TreeDepth) -> bool {
        match other {
            TreeDepth::All => false,
            TreeDepth::Depth(n) => self == n,
        }
    }
}

impl PartialOrd<TreeDepth> for usize {
    fn partial_cmp(&self, other: &TreeDepth) -> Option<Ordering> {
        match other {
            TreeDepth::All => Some(Ordering::Less),
            TreeDepth::Depth(n) => self.partial_cmp(n),
        }
    }
}
