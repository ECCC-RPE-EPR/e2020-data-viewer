use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Cell, Clear, Row, Table, TableState},
};

use super::{app::Mode, Component, Frame};
use crate::action::Action;

#[derive(Default)]
pub struct Help {
    pub previous_mode: Mode,
    pub state: TableState,
}

impl Help {
    pub fn init(&mut self) -> color_eyre::eyre::Result<()> {
        Ok(())
    }

    pub fn items(&self) -> Vec<Vec<String>> {
        let r = match self.previous_mode {
            Mode::Picker => {
                vec![
                    ["j / ↓", "Move down"],
                    ["k / ↑", "Move up"],
                    ["PageUp", "Go to top"],
                    ["PageDown", "Go to bottom"],
                    ["/", "Enter Fuzzy Find Mode"],
                    ["ESC", "Exit Fuzzy Find Mode"],
                    ["Enter", "Choose Current Selection"],
                    ["r", "Reload Data"],
                    ["q", "Quit"],
                    ["?", "Open Help"],
                ]
            }
            Mode::Viewer(_) => {
                vec![
                    ["h / ←", "Move left"],
                    ["j / ↓", "Move down"],
                    ["k / ↑", "Move up"],
                    ["l / →", "Move right"],
                    ["PageUp", "Go to top"],
                    ["PageDown", "Go to bottom"],
                    ["F1 / Shift+F1", "Cycle 1st dimension"],
                    ["F2 / Shift+F2", "Cycle 2nd dimension"],
                    ["F3 / Shift+F3", "Cycle 3rd dimension"],
                    ["F4 / Shift+F4", "Cycle 4rd dimension"],
                    ["F5 / Shift+F5", "Cycle 5th dimension"],
                    ["F6 / Shift+F6", "Cycle 6th dimension"],
                    ["F7 / Shift+F7", "Cycle 7th dimension"],
                    ["F8 / Shift+F8", "Cycle 8th dimension"],
                    ["F9 / Shift+F9", "Cycle 9th dimension"],
                    ["1 / Ctrl+1", "Cycle 1st dimension"],
                    ["2 / Ctrl+2", "Cycle 2nd dimension"],
                    ["3 / Ctrl+3", "Cycle 3rd dimension"],
                    ["4 / Ctrl+4", "Cycle 4rd dimension"],
                    ["5 / Ctrl+5", "Cycle 5th dimension"],
                    ["6 / Ctrl+6", "Cycle 6th dimension"],
                    ["7 / Ctrl+7", "Cycle 7th dimension"],
                    ["8 / Ctrl+8", "Cycle 8th dimension"],
                    ["9 / Ctrl+9", "Cycle 9th dimension"],
                    ["[ / ]", "Cycle 1st Axis"],
                    ["{ / }", "Cycle 2nd Axis"],
                    ["s", "Select mode"],
                    ["v", "Toggle current set in Select mode"],
                    ["t", "Toggle totals"],
                    [".", "Toggle formatting"],
                    ["ESC", "Close Viewer"],
                    ["?", "Open Help"],
                ]
            }
            _ => vec![],
        };
        r.iter()
            .map(|v| v.iter().map(|i| i.to_string()).collect())
            .collect()
    }

    pub fn next(&mut self) {
        if self.items().is_empty() {
            self.state.select(None)
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.items().len() - 1 {
                        self.items().len() - 1
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    pub fn previous(&mut self) {
        if self.items().is_empty() {
            self.state.select(None)
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        0
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }
}

impl Component for Help {
    fn handle_key_events(&mut self, key: KeyEvent) -> Option<Action> {
        let action = match key.code {
            KeyCode::Esc => Action::SwitchModeToPreviousMode,
            KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionNext,
            KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionPrevious,
            _ => return None,
        };
        Some(action)
    }

    fn update(&mut self, command: Action) -> Result<Option<Action>> {
        match command {
            Action::Refresh => self.init().unwrap(),
            Action::MoveSelectionNext => self.next(),
            Action::MoveSelectionPrevious => self.previous(),
            Action::SwitchModeToPicker => {
                return Ok(Some(Action::Refresh));
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) {
        f.render_widget(Clear, rect);
        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                "Help - Key Bindings",
                Style::default().add_modifier(Modifier::BOLD),
            )]))
            .title(Title::from("Press ESC to close.").alignment(Alignment::Right))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        f.render_widget(block, rect);
        let rows = self.items().into_iter().map(|item| {
            let cells: Vec<_> = item
                .iter()
                .enumerate()
                .map(|(i, c)| Line::from(c.clone()).alignment(Alignment::Left))
                .collect();
            Row::new(cells)
        });
        let table = Table::new(
            rows,
            [Constraint::Percentage(25), Constraint::Percentage(75)],
        )
        .header(
            Row::new(vec!["Key", "Action"])
                .bottom_margin(1)
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1);
        f.render_stateful_widget(
            table,
            rect.inner(&Margin {
                vertical: 2,
                horizontal: 3,
            }),
            &mut self.state,
        );
    }
}
