use approx::{abs_diff_eq, AbsDiffEq};
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ndarray::{prelude::*, s, Dimension, IxDyn, Slice, SliceInfo, SliceInfoElem};
use ratatui::{prelude::*, widgets::*};
use tui_input::{backend::crossterm::EventHandler, Input};

use super::{select::Select, summary::Summary, Component};
use crate::{action::Action, data::Data, trace_dbg};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Editing,
    Selection,
}

#[derive(Debug, Default)]
pub struct Viewer {
    pub file: String,
    pub name: String,
    pub focus: bool,
    pub data: Option<Data>,
    pub ncol: usize,
    pub nrow: usize,
    pub state: TableState,
    pub axis0: usize,
    pub axis1: usize,
    pub col: usize,
    pub row: usize,
    pub active_index: Vec<usize>,
    pub show_zeros_as_dashes: bool,
    pub input: Input,
    pub mode: Mode,
    pub summary: Summary,
    pub select: Select,
}

impl Viewer {
    pub fn initialize_state(&mut self) -> Result<()> {
        let data = self.data.as_ref().ok_or_else(|| {
            color_eyre::eyre::eyre!("Unable to extract HDF dataset from internal Option<Data>.")
        })?;
        if self.axis0 == self.axis1 {
            self.axis0 = data.ndims - 1;
            self.axis1 = 0;
        };
        self.ncol = data.set_data[self.axis0].len();
        self.nrow = data.set_data[self.axis1].len();
        // log::debug!("{:?}", data.set_data[self.axis1]);
        if self.active_index.is_empty() {
            self.active_index = vec![0; data.ndims];
        }
        self.summary.refresh(
            data.clone(),
            self.active_index.clone(),
            self.axis0,
            self.axis1,
        )?;
        self.select
            .refresh(data.set_data.clone(), data.set_names.clone());
        Ok(())
    }

    pub fn data(&mut self) -> Result<Vec<Vec<String>>> {
        if let Some(ref d) = self.data {
            let mut slices = Vec::new();
            for i in (0..d.ndims).rev() {
                if i == self.axis0 || i == self.axis1 {
                    slices.push(SliceInfoElem::Slice {
                        start: 0,
                        end: None,
                        step: 1,
                    });
                } else {
                    slices.push(SliceInfoElem::Index(self.active_index[i] as isize));
                }
            }
            log::debug!("{:?} {:?} = {:?}", self.axis0, self.axis1, &slices);
            let s = SliceInfo::<Vec<SliceInfoElem>, IxDyn, IxDyn>::try_from(slices)?;
            log::debug!("Start reading slice");
            let data = d.dataset.read_slice_2d(s)?;
            log::debug!("End reading slice");
            let data = if self.axis1 > self.axis0 {
                data.t().to_owned()
            } else {
                data
            };
            let (cols, rows) = data.dim();
            log::debug!("rows = {rows}, cols = {cols}");
            log::debug!("self.row = {}, self.col = {}", self.row, self.col);
            log::debug!("self.nrow = {}, self.ncol = {}", self.nrow, self.ncol);
            let totals_0 = data.sum_axis(Axis(0)).into_raw_vec();
            let totals_1 = data.sum_axis(Axis(1)).into_raw_vec();
            let vec_of_vecs = data.map_axis(ndarray::Axis(0), |row| row.to_vec()).to_vec();
            let mut vov: Vec<Vec<_>> = Vec::with_capacity(rows);
            for i in 0..=rows {
                if i == rows {
                    let mut v = totals_1[self.col..].to_vec();
                    v.insert(0, totals_0.iter().sum::<f64>());
                    vov.push(v);
                } else {
                    let mut v = vec_of_vecs[i][self.col..].to_vec();
                    v.insert(0, totals_0[i]);
                    vov.push(v);
                }
            }
            log::debug!(
                "vec_of_vecs: rows = {}, cols = {}",
                vec_of_vecs.len(),
                vec_of_vecs[0].len()
            );
            log::debug!("axis0 = {}, axis1 = {}", self.axis0, self.axis1);
            let vec_of_vecs: Vec<Vec<String>> = vov
                .iter()
                .map(|v| {
                    Vec::from_iter(v.iter().map(|f: &f64| {
                        if self.show_zeros_as_dashes && abs_diff_eq!(*f, 0.0) {
                            "-".to_string()
                        } else if self.show_zeros_as_dashes && f.fract() == 0.0 {
                            format!("{}", *f as i64)
                        } else {
                            format!("{:.2}", f)
                        }
                    }))
                })
                .collect();
            if let Some(first_size) = vec_of_vecs.first().map(|v| v.len()) {
                assert!(vec_of_vecs.iter().all(|vec| vec.len() == first_size));
            };
            Ok(vec_of_vecs)
        } else {
            Ok(vec![])
        }
    }

    pub fn reset(&mut self) {
        self.state = TableState::default();
        self.active_index = Vec::default();
        self.focus = true;
    }

    pub fn columns(&self) -> Vec<String> {
        let set_data = self.data.as_ref().unwrap().set_data.clone();
        let set_names = self.data.as_ref().unwrap().set_names.clone();
        let mut columns = set_data[self.axis0][self.col..self.ncol].to_vec();
        columns.insert(0, "Total".into());
        columns.insert(
            0,
            format!(
                "{}＼{}",
                set_names[self.axis1].clone(),
                set_names[self.axis0].clone()
            ),
        );
        columns
    }

    pub fn rows(&self) -> Vec<String> {
        let mut v = self.data.as_ref().unwrap().set_data[self.axis1][self.row..].to_vec();
        v.push("Total".into());
        v
    }

    pub fn constraints(&self, width: u16) -> Vec<Constraint> {
        let mut constraints = vec![Constraint::Length(20)];
        let mut total_width = 21;
        while total_width + 10 < width {
            constraints.push(Constraint::Length(9));
            total_width += 10;
        }
        constraints
    }

    pub fn move_top(&mut self) {
        if self.nrow == 0 {
            self.state.select(None)
        } else {
            self.state.select(Some(0))
        }
    }

    pub fn move_bottom(&mut self) {
        if self.nrow == 0 {
            self.state.select(None)
        } else {
            self.state.select(Some(self.nrow + 1));
        }
    }

    pub fn move_next(&mut self) {
        if self.nrow == 0 {
            self.state.select(None)
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i > self.nrow {
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

    pub fn move_previous(&mut self) {
        if self.nrow == 0 {
            self.state.select(None)
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.nrow
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    pub fn move_right(&mut self) {
        self.col += 1;
        self.col = self.col.min(self.ncol);
    }

    pub fn move_left(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn move_home(&mut self) {
        self.col = 0;
    }

    pub fn move_end(&mut self) {
        self.col = self.ncol;
    }

    pub fn increment_axis0(&mut self) {
        self.row = 0;
        self.col = 0;
        self.axis0 += 1;
        // cycle around to first
        if self.axis0 >= self.active_index.len() {
            self.axis0 = 0
        }
        // Never let axis0 == axis1
        if self.axis0 == self.axis1 {
            self.axis0 += 1;
            // cycle around to first
            if self.axis0 >= self.active_index.len() {
                self.axis0 = 0
            }
        }
    }

    pub fn increment_axis1(&mut self) {
        self.row = 0;
        self.col = 0;
        self.axis1 += 1;
        // cycle around to first
        if self.axis1 >= self.active_index.len() {
            self.axis1 = 0
        }
        // Never let axis1 == axis0
        if self.axis1 == self.axis0 {
            self.axis1 += 1;
            if self.axis1 >= self.active_index.len() {
                self.axis1 = 0
            }
        }
    }

    pub fn decrement_axis0(&mut self) {
        // cycle around to first
        if self.axis0 == 0 {
            self.axis0 = self.active_index.len() - 1
        } else {
            self.axis0 = self.axis0.saturating_sub(1);
        }
        // Never let axis0 == axis1
        if self.axis0 == self.axis1 {
            // cycle around to first
            if self.axis0 == 0 {
                self.axis0 = self.active_index.len() - 1
            } else {
                self.axis0 = self.axis0.saturating_sub(1);
            }
        }
    }

    pub fn decrement_axis1(&mut self) {
        if self.axis1 == 0 {
            self.axis1 = self.active_index.len() - 1
        } else {
            self.axis1 = self.axis1.saturating_sub(1);
        }
        // cycle around to first
        // Never let axis1 == axis0
        if self.axis1 == self.axis0 {
            if self.axis1 == 0 {
                self.axis1 = self.active_index.len() - 1
            } else {
                self.axis1 = self.axis1.saturating_sub(1);
            }
            // cycle around to first
        }
    }

    pub fn increment_index(&mut self, i: usize) -> Result<()> {
        if i >= self.active_index.len() {
            let s = &self.active_index;
            log::error!("Trying to modify index position `{i}` in array of shape `{s:?}`.");
        } else {
            self.active_index[i] += 1;
            if self.active_index[i] >= self.data.as_ref().unwrap().set_data[i].len() {
                self.active_index[i] = 0;
            }
        }
        Ok(())
    }

    pub fn decrement_index(&mut self, i: usize) -> Result<()> {
        if i >= self.active_index.len() {
            let s = &self.active_index;
            log::error!("Trying to modify index position `{i}` in array of shape `{s:?}`.");
        } else if self.active_index[i] == 0 {
            self.active_index[i] = self.data.as_ref().unwrap().set_data[i]
                .len()
                .saturating_sub(1);
        } else {
            self.active_index[i] = self.active_index[i].saturating_sub(1);
        }
        Ok(())
    }
}

impl Component for Viewer {
    fn init(&mut self) -> Result<()> {
        self.focus = true;
        self.show_zeros_as_dashes = true;

        self.data = Some(Data::new(self.file.clone().into(), self.name.clone())?);
        self.axis1 = 0;
        self.axis0 = self.data.as_ref().unwrap().ndims - 1;

        self.initialize_state().unwrap();

        Ok(())
    }

    fn handle_key_events(&mut self, key: crossterm::event::KeyEvent) -> Option<Action> {
        let action = match self.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('?') => Action::SwitchModeToHelp,
                    KeyCode::Char('q') => Action::Quit,
                    KeyCode::F(1) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(0)
                    }
                    KeyCode::F(2) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(1)
                    }
                    KeyCode::F(3) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(2)
                    }
                    KeyCode::F(4) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(3)
                    }
                    KeyCode::F(5) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(4)
                    }
                    KeyCode::F(6) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(5)
                    }
                    KeyCode::F(7) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(6)
                    }
                    KeyCode::F(8) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(7)
                    }
                    KeyCode::F(9) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        Action::PreviousAxis(8)
                    }
                    KeyCode::F(1) if key.modifiers.is_empty() => Action::NextAxis(0),
                    KeyCode::F(2) if key.modifiers.is_empty() => Action::NextAxis(1),
                    KeyCode::F(3) if key.modifiers.is_empty() => Action::NextAxis(2),
                    KeyCode::F(4) if key.modifiers.is_empty() => Action::NextAxis(3),
                    KeyCode::F(5) if key.modifiers.is_empty() => Action::NextAxis(4),
                    KeyCode::F(6) if key.modifiers.is_empty() => Action::NextAxis(5),
                    KeyCode::F(7) if key.modifiers.is_empty() => Action::NextAxis(6),
                    KeyCode::F(8) if key.modifiers.is_empty() => Action::NextAxis(7),
                    KeyCode::F(9) if key.modifiers.is_empty() => Action::NextAxis(8),
                    KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(0)
                    }
                    KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(1)
                    }
                    KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(2)
                    }
                    KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(3)
                    }
                    KeyCode::Char('5') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(4)
                    }
                    KeyCode::Char('6') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(5)
                    }
                    KeyCode::Char('7') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(6)
                    }
                    KeyCode::Char('8') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(7)
                    }
                    KeyCode::Char('9') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        Action::PreviousAxis(8)
                    }
                    KeyCode::Char('1') if key.modifiers.is_empty() => Action::NextAxis(0),
                    KeyCode::Char('2') if key.modifiers.is_empty() => Action::NextAxis(1),
                    KeyCode::Char('3') if key.modifiers.is_empty() => Action::NextAxis(2),
                    KeyCode::Char('4') if key.modifiers.is_empty() => Action::NextAxis(3),
                    KeyCode::Char('5') if key.modifiers.is_empty() => Action::NextAxis(4),
                    KeyCode::Char('6') if key.modifiers.is_empty() => Action::NextAxis(5),
                    KeyCode::Char('7') if key.modifiers.is_empty() => Action::NextAxis(6),
                    KeyCode::Char('8') if key.modifiers.is_empty() => Action::NextAxis(7),
                    KeyCode::Char('9') if key.modifiers.is_empty() => Action::NextAxis(8),
                    // KeyCode::Char('s') => Action::EnterSubset,
                    KeyCode::Char(']') => Action::IncrementAxis(0),
                    KeyCode::Char('}') => Action::IncrementAxis(1),
                    KeyCode::Char('[') => Action::DecrementAxis(0),
                    KeyCode::Char('{') => Action::DecrementAxis(1),
                    KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionNext,
                    KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionPrevious,
                    KeyCode::Char('h') | KeyCode::Left => Action::MoveSelectionLeft,
                    KeyCode::Char('l') | KeyCode::Right => Action::MoveSelectionRight,
                    KeyCode::Home => Action::MoveSelectionHome,
                    KeyCode::End => Action::MoveSelectionEnd,
                    KeyCode::PageUp => Action::MoveSelectionTop,
                    KeyCode::PageDown => Action::MoveSelectionBottom,
                    KeyCode::Enter => Action::SubmitSelection,
                    KeyCode::Esc => Action::Close,
                    KeyCode::Char('.') => Action::ToggleFormattedData,
                    _ => return None,
                }
            }
            Mode::Editing => match key.code {
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Enter => Action::EnterNormal,
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    return None;
                }
            },
            Mode::Selection => self.select.handle_key_events(key)?,
        };
        Some(action)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match self.mode {
            Mode::Selection => {
                match action {
                    Action::EnterNormal => {
                        self.mode = Mode::Normal;
                        self.init()?;
                    }
                    _ => {
                        self.select.update(action)?;
                    }
                };
            }
            _ => {
                match action {
                    Action::SwitchModeToViewer(_) => {
                        self.init()?;
                        return Ok(Some(Action::MoveSelectionNext));
                    }
                    Action::ToggleFormattedData => {
                        self.show_zeros_as_dashes = !self.show_zeros_as_dashes;
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionNext => {
                        self.move_next();
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionPrevious => {
                        self.move_previous();
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionLeft => {
                        self.move_left();
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionRight => {
                        self.move_right();
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionTop => {
                        self.move_top();
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionBottom => {
                        self.move_bottom();
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionHome => {
                        self.move_home();
                        self.initialize_state().unwrap();
                    }
                    Action::MoveSelectionEnd => {
                        self.move_end();
                        self.initialize_state().unwrap();
                    }
                    Action::NextAxis(i) => {
                        self.increment_index(i)?;
                        self.initialize_state().unwrap();
                    }
                    Action::PreviousAxis(i) => {
                        self.decrement_index(i)?;
                        self.initialize_state().unwrap();
                    }
                    Action::IncrementAxis(i) => {
                        if i == 1 {
                            // log::debug!("Incrementing axis 0");
                            self.increment_axis0();
                        } else {
                            // log::debug!("Incrementing axis 1");
                            self.increment_axis1();
                        }
                        self.initialize_state().unwrap();
                    }
                    Action::DecrementAxis(i) => {
                        if i == 1 {
                            // log::debug!("Decrementing axis 0");
                            self.decrement_axis0();
                        } else {
                            // log::debug!("Decrementing axis 1");
                            self.decrement_axis1();
                        }
                        self.initialize_state().unwrap();
                    }
                    Action::EnterInsert => self.mode = Mode::Editing,
                    Action::EnterNormal => {
                        self.mode = Mode::Normal;
                        self.initialize_state().unwrap();
                    }
                    Action::Close => {
                        self.reset();
                        return Ok(Some(Action::SwitchModeToPicker));
                    }
                    Action::EnterSubset => {
                        self.mode = Mode::Selection;
                    }
                    _ => return Ok(None),
                };
            }
        };
        Ok(None)
    }

    fn draw(&mut self, f: &mut super::Frame<'_>, rect: Rect) {
        let summary_constraint = if self.active_index.len() > 2 {
            Constraint::Min(self.active_index.len() as u16 + 5)
        } else {
            Constraint::Min(0)
        };

        let rects = Layout::default()
            .constraints([summary_constraint, Constraint::Percentage(100)].as_ref())
            .split(rect);
        self.summary.draw(f, rects[0]);

        log::debug!("getting data");
        let items = self.data().unwrap();
        log::debug!("got data");
        log::debug!("items.len() = {}", items.len());
        let columns = self.columns();
        log::debug!("columns.len() = {}", columns.len());
        let rows = self.rows();
        log::debug!("rows.len() = {}", rows.len());
        let constraints = self.constraints(rect.width);

        let header_cells = columns.iter().enumerate().map(|(i, h)| {
            if i == 0 {
                Cell::from(h.clone()).style(Style::default().fg(Color::Yellow))
            } else {
                Cell::from(Line::from(h.clone()).alignment(Alignment::Right))
                    .style(Style::default().add_modifier(Modifier::BOLD))
            }
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);
        let rows = items.iter().enumerate().map(|(i, item)| {
            let height = 1;
            let mut cells: Vec<_> = item
                .iter()
                .enumerate()
                .map(|(j, c)| Cell::from(Line::from(c.clone()).alignment(Alignment::Right)))
                .collect();
            cells.insert(
                0,
                Cell::from(Line::from(rows[i].clone()).alignment(Alignment::Left))
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            );
            Row::new(cells).height(height as u16)
        });
        let highlight_symbol = if self.focus { " \u{2022} " } else { "" };
        let nrows = rows.len();
        let table = Table::new(rows, constraints)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default())
                    .title("Viewer")
                    .border_style(if self.focus {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(highlight_symbol);

        f.render_stateful_widget(table, rects[1], &mut self.state);

        // let width = rects[2].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        // let scroll = self.input.visual_scroll(width as usize);
        // let input = Paragraph::new(self.input.value())
        //   .style(match self.mode {
        //     Mode::Editing => Style::default().fg(Color::Yellow),
        //     _ => Style::default(),
        //   })
        //   .scroll((0, scroll as u16))
        //   .block(Block::default().borders(Borders::ALL).title(Line::from(vec![Span::raw("Select ")])));
        // f.render_widget(input, rects[2]);

        if self.mode == Mode::Selection {
            let tabs_area = rect.inner(&Margin {
                vertical: 4,
                horizontal: 4,
            });
            self.select.draw(f, tabs_area);
        }
    }
}
