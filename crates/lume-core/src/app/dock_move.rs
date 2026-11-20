//! Moving a dock item along the dock, as "Move" in its context menu starts it: the neighbors make
//! room for an arrow on each side, Left and Right carry the item, OK keeps the new place and
//! Back puts it back where it was. The order is written only when it is kept.

use super::{ids, Effect, Key, LumeApp, PendingFocus};
use crate::ui::focus::FocusId;

#[derive(Clone, Debug, PartialEq, Eq)]
enum MoveTarget {
    Favorite(String),
    AllApps,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DockMove {
    target: MoveTarget,
    /// The favorites order before the move, for Back.
    original_favorites: Vec<String>,
    /// The "All apps" dock slot before the move, for Back.
    original_all_apps_index: Option<usize>,
}

impl LumeApp {
    /// The favorite being moved, if any (for tests).
    #[doc(hidden)]
    pub fn moving_favorite(&self) -> Option<&str> {
        match self.dock_move.as_ref().map(|m| &m.target) {
            Some(MoveTarget::Favorite(pkg)) => Some(pkg.as_str()),
            _ => None,
        }
    }

    /// Whether the fixed "All apps" dock item is currently being moved (for tests).
    #[doc(hidden)]
    pub fn moving_dock_all_apps(&self) -> bool {
        matches!(
            self.dock_move.as_ref().map(|m| &m.target),
            Some(MoveTarget::AllApps)
        )
    }

    pub(super) fn start_dock_move(&mut self, package: &str) {
        self.dock_move = Some(DockMove {
            target: MoveTarget::Favorite(package.to_string()),
            original_favorites: self.config.favorites.clone(),
            original_all_apps_index: self.config.all_apps_dock_index,
        });
        self.animate_move_room(true);
    }

    pub(super) fn start_dock_move_all_apps(&mut self) {
        if !self.config.has_dock_all_apps() || self.config.favorites.is_empty() {
            return;
        }
        self.dock_move = Some(DockMove {
            target: MoveTarget::AllApps,
            original_favorites: self.config.favorites.clone(),
            original_all_apps_index: self.config.all_apps_dock_index,
        });
        self.animate_move_room(true);
    }

    fn dock_move_slot(&self, target: &MoveTarget) -> Option<usize> {
        match target {
            MoveTarget::AllApps => self
                .config
                .has_dock_all_apps()
                .then(|| self.config.effective_all_apps_dock_index()),
            MoveTarget::Favorite(package) => {
                let fav_index = self.config.favorites.iter().position(|p| p == package)?;
                if self.config.has_dock_all_apps() {
                    let all_idx = self.config.effective_all_apps_dock_index();
                    Some(if fav_index < all_idx {
                        fav_index
                    } else {
                        fav_index + 1
                    })
                } else {
                    Some(fav_index)
                }
            }
        }
    }

    fn dock_move_focus_id(&self, target: &MoveTarget) -> Option<FocusId> {
        match target {
            MoveTarget::AllApps => Some(ids::DOCK_ALL_APPS),
            MoveTarget::Favorite(package) => self
                .config
                .favorites
                .iter()
                .position(|p| p == package)
                .map(favorite_id),
        }
    }

    /// A key while a dock item is being moved: the focus stays on it.
    pub(super) fn dock_move_key(&mut self, key: Key) -> Vec<Effect> {
        let Some(target) = self.dock_move.as_ref().map(|m| m.target.clone()) else {
            return vec![];
        };
        let Some(slot) = self.dock_move_slot(&target) else {
            self.end_dock_move_quietly();
            return vec![];
        };
        match key {
            Key::Left | Key::Right => {
                let count = self.config.dock_slot_count();
                let to = match key {
                    Key::Left => slot.checked_sub(1),
                    _ => Some(slot + 1).filter(|&to| to < count),
                };
                if let Some(to) = to {
                    self.begin_dock_transition(Some(to));
                    if self.config.move_dock_slot(slot, to) {
                        if let Some(id) = self.dock_move_focus_id(&target) {
                            self.pending_focus = Some(PendingFocus::Id(id));
                        }
                    }
                }
                vec![]
            }
            Key::Ok => {
                let state = self.dock_move.take();
                self.animate_move_room(false);
                let kept = state.is_some_and(|m| {
                    m.original_favorites != self.config.favorites
                        || m.original_all_apps_index != self.config.all_apps_dock_index
                });
                if kept {
                    vec![Effect::SaveConfig]
                } else {
                    vec![]
                }
            }
            Key::Back | Key::Home => self.cancel_dock_move(),
            Key::Up | Key::Down | Key::Menu => vec![],
        }
    }

    /// Puts the dock items back in the order they had before the move.
    fn cancel_dock_move(&mut self) -> Vec<Effect> {
        let Some(state) = self.dock_move.take() else {
            return vec![];
        };
        self.animate_move_room(false);
        if state.original_favorites == self.config.favorites
            && state.original_all_apps_index == self.config.all_apps_dock_index
        {
            return vec![];
        }
        let original: Vec<String> = state
            .original_favorites
            .into_iter()
            .filter(|p| self.catalog.contains(p))
            .collect();
        let original_all = state
            .original_all_apps_index
            .map(|idx| idx.min(original.len()));
        self.config.favorites = original;
        self.config.all_apps_dock_index = original_all;
        let back_slot = self.dock_move_slot(&state.target);
        self.begin_dock_transition(back_slot);
        if let Some(id) = self.dock_move_focus_id(&state.target) {
            self.pending_focus = Some(PendingFocus::Id(id));
        }
        // Written although it is the old order: an uninstall may have saved the moved one meanwhile.
        vec![Effect::SaveConfig]
    }

    /// Leaves the move when the screen changes under it (Home, a return from an app), with the
    /// dock items in their old order and without a slide.
    pub(super) fn end_dock_move_quietly(&mut self) {
        if let Some(state) = self.dock_move.take() {
            let original: Vec<String> = state
                .original_favorites
                .into_iter()
                .filter(|p| self.catalog.contains(p))
                .collect();
            self.config.all_apps_dock_index = state
                .original_all_apps_index
                .map(|idx| idx.min(original.len()));
            self.config.favorites = original;
            self.anim.set(super::Anim::DockRoom, 0.0);
        }
    }

    /// The catalog changed: a move whose favorite was uninstalled ends.
    pub(super) fn check_dock_move(&mut self) {
        let gone = self.dock_move.as_ref().is_some_and(|m| match &m.target {
            MoveTarget::Favorite(pkg) => !self.config.favorites.contains(pkg),
            MoveTarget::AllApps => {
                !self.config.has_dock_all_apps() || self.config.favorites.is_empty()
            }
        });
        if gone {
            self.end_dock_move_quietly();
        }
    }
}

fn favorite_id(index: usize) -> FocusId {
    ids::FAVORITE_BASE + index as FocusId
}
