use std::{
    collections::HashSet,
    io::Stderr,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use color_eyre::eyre::{anyhow, eyre, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use hdf5::types::{FixedUnicode, VarLenUnicode};
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tui_input::{backend::crossterm::EventHandler, Input};

use super::{Component, Frame};
use crate::{action::Action, data::Data, runner::Runner};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Editing,
}

#[derive(Default, Debug)]
pub struct Picker {
    pub file: String,
    pub state: TableState,
    pub columns: Vec<String>,
    pub constraints: Vec<Constraint>,
    pub focus: bool,
    pub bold_first_row_col: bool,
    pub bold_first_row: bool,
    pub marked: HashSet<usize>,
    pub groups: Vec<String>,
    pub datasets: Arc<Mutex<Vec<Data>>>,
    pub loading_status: Arc<AtomicBool>,
    pub ndatasets: Arc<AtomicUsize>,
    pub loading: usize,
    pub input: Input,
    pub mode: Mode,
    pub task: Option<JoinHandle<()>>,
    pub cancellation_token: Option<CancellationToken>,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub filtered_items: Vec<Vec<String>>,
}

impl Picker {
    pub fn init(&mut self) -> Result<()> {
        log::debug!("Inside dataset picker init");
        self.focus = true;
        self.bold_first_row = true;
        log::debug!("Before read self.get_datasets()");
        self.get_datasets();
        log::debug!("After read self.get_datasets()");
        self.refresh();
        Ok(())
    }

    pub fn tick(&mut self) {
        let filter = self.input.value().to_lowercase();
        let filter_words = filter.split_whitespace().collect::<Vec<_>>();
        self.filtered_items = self
            .datasets
            .lock()
            .unwrap()
            .iter()
            .filter(|d| {
                filter_words
                    .iter()
                    .all(|word| d.name.to_lowercase().contains(word))
            })
            .map(|d| {
                vec![
                    format!("'{}'", d.name.clone()),
                    format!("{}", d.set_names.join(", ")),
                    format!("{}", d.shape.iter().map(|i| i.to_string()).join(", ")),
                    format!("{}", d.ndims),
                    d.units.clone(),
                    d.doc.clone(),
                ]
            })
            .collect();
    }

    pub fn reset(&mut self) {
        self.state = TableState::default();
        self.columns = Vec::default();
        self.constraints = Vec::default();
        self.focus = true;
    }

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

    pub fn top(&mut self) {
        if self.filtered_items().is_empty() {
            self.state.select(None)
        } else {
            self.state.select(Some(0))
        }
    }

    pub fn bottom(&mut self) {
        if self.filtered_items().is_empty() {
            self.state.select(None)
        } else {
            self.state.select(Some(self.filtered_items().len() - 1));
        }
    }

    pub fn next(&mut self) {
        if self.filtered_items().is_empty() {
            self.state.select(None)
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.filtered_items().len() - 1 {
                        0
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
        if self.filtered_items().is_empty() {
            self.state.select(None)
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.filtered_items().len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    pub fn get_datasets(&mut self) {
        log::debug!("-------- Reading from {}", self.file);
        let datasets = self.datasets.clone();
        let file = self.file.clone();
        let loading_status = self.loading_status.clone();
        let ndatasets = self.ndatasets.clone();
        self.cancellation_token = Some(CancellationToken::new());
        let _cancellation_token = self.cancellation_token.clone().unwrap();
        let _action_tx = self.action_tx.clone();
        self.task = Some(tokio::spawn(async move {
            datasets.lock().unwrap().drain(0..);
            loading_status.store(true, Ordering::SeqCst);
            let mut names = vec![];
            let f = hdf5::File::open(&file).unwrap();
            for group in f.member_names().unwrap() {
                for dataset in f.group(&group).unwrap().member_names().unwrap() {
                    names.push(format!("{group}/{dataset}"));
                }
            }
            ndatasets.store(names.len(), Ordering::SeqCst);
            let mut count = 0;
            for name in names {
                if let Ok(d) = Data::new(file.clone().into(), name) {
                    datasets.lock().unwrap().push(d);
                    count += 1;
                }
                if _cancellation_token.is_cancelled() {
                    break;
                }
            }
            ndatasets.store(count, Ordering::SeqCst);
            if let Some(action_tx) = _action_tx {
                action_tx.send(Action::Tick).unwrap_or_default();
                action_tx
                    .send(Action::MoveSelectionNext)
                    .unwrap_or_default();
            }
            loading_status.store(false, Ordering::SeqCst);
            log::debug!("Finished reading from {}", file);
        }));
    }

    pub fn cancel(&mut self) {
        if let Some(ref t) = self.cancellation_token {
            t.cancel();
            if let Some(ref t) = self.task {
                while !t.is_finished() {
                    std::thread::sleep(Duration::from_millis(50))
                }
            }
        }
    }

    pub fn filtered_items(&self) -> Vec<Vec<String>> {
        self.filtered_items.clone()
    }

    pub fn refresh(&mut self) {
        log::debug!(
            "list of datasets = {:?}",
            self.datasets.lock().unwrap().len()
        );

        self.columns = ["Name", "Dims", "Shape", "N", "Units", "Documentation"]
            .map(String::from)
            .to_vec();
        self.constraints = vec![
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(5),
            Constraint::Percentage(5),
            Constraint::Percentage(20),
        ];
        if self.datasets.lock().unwrap().len() > 0 {
            if self.state.selected().is_none() {
                self.state.select(Some(0))
            }
        } else {
            self.state.select(None)
        }
        match self.mode {
            Mode::Normal => self.focus = true,
            Mode::Editing => self.focus = false,
        }
    }

    pub fn select(&mut self, selection: usize) -> usize {
        let items = self.filtered_items();
        let name = items[selection][0]
            .strip_prefix('\'')
            .unwrap()
            .strip_suffix('\'')
            .unwrap();
        let (i, d) = self
            .datasets
            .lock()
            .unwrap()
            .iter()
            .find_position(|d| d.name == name)
            .ok_or_else(|| anyhow!("Unable to get selection. Something went wrong"))
            .unwrap();
        log::info!("Selecting {name}");
        i
    }
}

impl Component for Picker {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Option<Action> {
        log::debug!("key: {key:?}");
        let cmd = match self.mode {
            Mode::Normal => match key.code {
                KeyCode::Char('q') => Action::Quit,
                KeyCode::Char('/') => Action::EnterInsert,
                KeyCode::Char('?') => Action::SwitchModeToHelp,
                KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionNext,
                KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionPrevious,
                KeyCode::Char('h') | KeyCode::Left => Action::MoveSelectionLeft,
                KeyCode::Char('l') | KeyCode::Right => Action::MoveSelectionRight,
                KeyCode::Char('g') | KeyCode::PageUp => Action::MoveSelectionTop,
                KeyCode::Char('G') | KeyCode::PageDown => Action::MoveSelectionBottom,
                KeyCode::Char('r') => Action::ReloadData,
                KeyCode::Char('v') => Action::ToggleSelection,
                KeyCode::Home => Action::MoveSelectionHome,
                KeyCode::End => Action::MoveSelectionEnd,
                KeyCode::Enter => Action::SubmitSelection,
                KeyCode::Esc => Action::Close,
                _ => return None,
            },
            Mode::Editing => match key.code {
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Enter => Action::EnterNormal,
                _ => {
                    self.input.handle_event(&Event::Key(key));
                    Action::Refresh
                }
            },
        };
        Some(cmd)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Quit => {
                if let Some(ref t) = self.task {
                    t.abort()
                }
            }
            Action::MoveSelectionNext => self.next(),
            Action::MoveSelectionPrevious => self.previous(),
            Action::MoveSelectionTop => self.top(),
            Action::MoveSelectionBottom => self.bottom(),
            Action::ReloadData => {
                self.cancel();
                self.get_datasets();
            }
            Action::EnterInsert => {
                self.mode = Mode::Editing;
                return Ok(Some(Action::Refresh));
            }
            Action::EnterNormal => {
                self.mode = Mode::Normal;
                return Ok(Some(Action::Refresh));
            }
            Action::SubmitSelection => {
                if let Some(selection) = self.state.selected() {
                    let dataset_index = self.select(selection);
                    return Ok(Some(Action::SwitchModeToViewer(dataset_index)));
                }
            }
            Action::Refresh => self.refresh(),
            Action::SwitchModeToPicker => {
                // self.input.set_value("");
                return Ok(Some(Action::Refresh));
            }
            Action::ToggleSelection => self.mark(self.state.selected()),
            Action::Tick => self.tick(),
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame, rect: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100), Constraint::Min(3)].as_ref())
            .split(rect);
        let header_cells = self.columns.iter().enumerate().map(|(i, h)| {
            if i == 0 {
                if self.bold_first_row_col || self.bold_first_row {
                    Cell::from(h.clone()).style(Style::default().add_modifier(Modifier::BOLD))
                } else {
                    Cell::from(h.clone()).style(Style::default())
                }
            } else if self.bold_first_row {
                Cell::from(h.clone()).style(Style::default().add_modifier(Modifier::BOLD))
            } else {
                Cell::from(h.clone()).style(Style::default())
            }
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);
        let items: Vec<Vec<String>> = self.filtered_items();
        let rows = items.iter().enumerate().map(|(i, item)| {
            let height = 1;
            let style = if self.contains(i) {
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let cells = item
                .iter()
                .enumerate()
                .map(|(j, c)| Cell::from(c.clone()).style(style));
            Row::new(cells).height(height as u16)
        });
        let highlight_symbol = if self.focus { " \u{2022} " } else { "" };
        let loading_status = if self.loading_status.load(Ordering::SeqCst) {
            format!(
                "Scanning {}/{}",
                self.datasets.lock().unwrap().len(),
                self.ndatasets.load(Ordering::SeqCst)
            )
        } else {
            format!(
                "{}/{}",
                self.state.selected().unwrap_or_default() + 1,
                self.ndatasets.load(Ordering::SeqCst)
            )
        };
        let table = Table::new(rows, &self.constraints)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Picker")
                    .title(block::Title::from(loading_status).alignment(Alignment::Right))
                    .border_style(if self.focus {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    }),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(highlight_symbol)
            .highlight_spacing(HighlightSpacing::Always);

        f.render_stateful_widget(table, rects[0], &mut self.state);

        if let Some(i) = self.state.selected() {
            let mut state = ScrollbarState::default()
                .position(i)
                .content_length(items.len());
            f.render_stateful_widget(
                Scrollbar::default().track_symbol(Some("║")),
                rects[0],
                &mut state,
            );
        }
        let width = rects[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        let input = Paragraph::new(self.input.value())
            .scroll((0, scroll as u16))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "Fuzzy Find ".into(),
                        "(Press ".into(),
                        "/".bold(),
                        " to start, ".into(),
                        "ESC".bold(),
                        " to finish)".into(),
                    ]))
                    .border_style(match self.mode {
                        Mode::Editing => Style::default().fg(Color::Yellow),
                        _ => Style::default().add_modifier(Modifier::DIM),
                    }),
            );
        f.render_widget(input, rects[1]);
        if self.mode == Mode::Editing {
            f.set_cursor(
                (rects[1].x + 1 + self.input.cursor() as u16).min(rects[1].x + rects[1].width - 2),
                rects[1].y + 1,
            )
        }
    }
}
