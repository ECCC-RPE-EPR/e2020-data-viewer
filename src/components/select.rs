use std::collections::HashSet;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Row, Table, Tabs},
};
use tracing::debug;

use super::{app::Mode, Component};
use crate::action::Action;

#[derive(Debug, Clone, Default)]
pub struct MultipleSelectionListState {
    marked: HashSet<usize>,
}

impl MultipleSelectionListState {
    pub fn contains(&self, i: usize) -> bool {
        self.marked.contains(&i)
    }

    pub fn marked(&self) -> std::collections::hash_set::Iter<usize> {
        self.marked.iter()
    }

    pub fn mark(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            self.marked.insert(i);
        }
    }

    pub fn unmark(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            self.marked.remove(&i);
        }
    }

    pub fn toggle(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            if !self.marked.insert(i) {
                self.marked.remove(&i);
            }
        }
    }

    pub fn clear(&mut self) {
        self.marked.drain().for_each(drop);
    }
}

#[derive(Default, Debug)]
pub struct StatefulList {
    pub list_state: ListState,
    pub multiple_selection_state: MultipleSelectionListState,
    pub items: Vec<String>,
}

impl StatefulList {
    pub fn with_items(items: Vec<String>) -> StatefulList {
        StatefulList {
            multiple_selection_state: MultipleSelectionListState::default(),
            list_state: ListState::default(),
            items,
        }
    }

    pub fn selected(&mut self) -> Vec<usize> {
        if self.multiple_selection_state.marked.is_empty() {
            self.multiple_selection_state.mark(Some(0));
        };
        let mut s = self
            .multiple_selection_state
            .marked
            .iter()
            .cloned()
            .collect::<Vec<usize>>();
        s.sort();
        s
    }

    pub fn toggle(&mut self) {
        self.multiple_selection_state
            .toggle(self.list_state.selected())
    }

    pub fn toggle_all(&mut self) {
        for i in 0..self.items.len() {
            self.multiple_selection_state.toggle(Some(i));
        }
    }

    pub fn mark_all(&mut self) {
        for i in 0..self.items.len() {
            self.multiple_selection_state.mark(Some(i));
        }
    }

    pub fn unmark_all(&mut self) {
        self.multiple_selection_state.clear();
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

#[derive(Default, Debug)]
pub struct Select {
    pub active_sets_state: Vec<StatefulList>,
    pub set_names: Vec<String>,
    current_set: usize,
}

impl Select {
    pub fn init(&mut self) -> Result<()> {
        for i in 0..self.active_sets_state.len() {
            self.active_sets_state[i].list_state.select(Some(0));
            self.active_sets_state[i].mark_all();
            debug!(
                "active_sets[{i}] = {:?}",
                self.active_sets_state[i].selected()
            );
        }
        Ok(())
    }

    pub fn next_element(&mut self) {
        self.active_sets_state[self.current_set].next()
    }

    pub fn previous_element(&mut self) {
        self.active_sets_state[self.current_set].previous()
    }

    pub fn next_set(&mut self) {
        self.current_set += 1;
        if self.current_set >= self.set_names.len() {
            self.current_set = 0
        }
    }

    pub fn previous_set(&mut self) {
        if self.current_set == 0 {
            self.current_set = self.set_names.len() - 1
        } else {
            self.current_set = self.current_set.saturating_sub(1);
        }
    }

    pub fn toggle(&mut self) {
        self.active_sets_state[self.current_set].toggle()
    }

    pub fn toggle_all(&mut self) {
        self.active_sets_state[self.current_set].toggle_all()
    }

    pub fn refresh(&mut self, set_data: Vec<Vec<String>>, set_names: Vec<String>) {
        self.active_sets_state = set_data
            .iter()
            .cloned()
            .map(StatefulList::with_items)
            .collect();
        self.set_names.clone_from(&set_names);
    }
}

impl Component for Select {
    fn handle_key_events(&mut self, key: KeyEvent) -> Option<Action> {
        let action = match key.code {
            KeyCode::Esc => Action::EnterNormal,
            KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionNext,
            KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionPrevious,
            KeyCode::Char('h') | KeyCode::Left => Action::MoveSelectionLeft,
            KeyCode::Char('l') | KeyCode::Right => Action::MoveSelectionRight,
            KeyCode::Char('V') => Action::ToggleAllSelection,
            KeyCode::Char('v') => Action::ToggleSelection,
            _ => return None,
        };
        Some(action)
    }

    fn update(&mut self, command: Action) -> Result<Option<Action>> {
        match command {
            Action::MoveSelectionNext => self.next_element(),
            Action::MoveSelectionPrevious => self.previous_element(),
            Action::MoveSelectionLeft => self.previous_set(),
            Action::MoveSelectionRight => self.next_set(),
            Action::ToggleSelection => self.toggle(),
            Action::ToggleAllSelection => self.toggle_all(),
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut super::Frame<'_>, rect: Rect) {
        f.render_widget(Clear, rect);
        let titles = self.set_names.iter().cloned().map(Line::from).collect_vec();
        let t = Tabs::new(titles)
            .block(
                Block::default()
                    .title(Line::from(vec![
                        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "◄ or ►",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to switch axis, ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "v",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to toggle values, ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "ESC",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Gray),
                        ),
                        Span::styled(" to close.", Style::default().fg(Color::DarkGray)),
                    ]))
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::White))
            .select(self.current_set)
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider(symbols::DOT);
        f.render_widget(t, rect);

        let items: Vec<ListItem> = self.active_sets_state[self.current_set]
            .items
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, s)| {
                let c = if self.active_sets_state[self.current_set]
                    .multiple_selection_state
                    .contains(i)
                {
                    "\u{2714} ".to_string()
                } else {
                    "  ".to_string()
                };
                let lines = vec![Line::from(c + &s)];
                ListItem::new(lines).style(Style::default())
            })
            .collect();
        let items = List::new(items)
            .block(Block::default())
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("→ ");

        f.render_stateful_widget(
            items,
            rect.inner(&Margin {
                vertical: 3,
                horizontal: 5,
            }),
            &mut self.active_sets_state[self.current_set].list_state,
        );
    }
}
