use std::{path::PathBuf, time::Duration};

use color_eyre::eyre::{bail, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph},
};

use crate::{
    action::Action,
    components::{help::Help, picker::Picker, viewer::Viewer, Component, Frame},
    data::Data,
    trace_dbg, tui,
    tui::{key_event_to_string, Event},
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Picker,
    Viewer(String),
    Waiting,
    Help,
}

#[derive(Default)]
pub struct App {
    pub mode: Mode,
    pub previous_mode: Mode,
    pub file: String,
    pub picker: Picker,
    pub viewer: Viewer,
    pub help: Help,
    pub last_event: String,
}

impl App {
    pub fn new(file: String, dataset: Option<String>) -> Result<Self> {
        if !PathBuf::from(file.clone()).exists() {
            return Err(color_eyre::eyre::eyre!("Unable to find {file:?}"));
        }
        let mut s = Self {
            file,
            ..Default::default()
        };
        if let Some(name) = dataset {
            if hdf5::File::open(s.file.clone())
                .expect("Unable to find file")
                .dataset(&name)
                .is_ok()
            {
                s.mode = Mode::Viewer(name);
                s.init().unwrap();
            } else {
                return Err(color_eyre::eyre::eyre!(
                    "Unable to load {:?} from {:?}. Are you sure {:?} exists in the file?",
                    name,
                    s.file.clone(),
                    name
                ));
            }
        }
        Ok(s)
    }

    pub fn quit(&mut self) {
        self.picker.cancel();
    }

    pub fn tick(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Component for App {
    fn init(&mut self) -> Result<()> {
        log::debug!("********** Inside App.init() **************");
        self.picker.file.clone_from(&self.file);
        self.viewer.file.clone_from(&self.file);
        match self.mode {
            Mode::Picker => self.picker.init(),
            Mode::Viewer(ref s) => {
                self.viewer.name.clone_from(s);
                self.viewer.init()
            }
            Mode::Help => {
                self.help.previous_mode = self.previous_mode.clone();
                self.help.init()
            }
            _ => Ok(()),
        }
    }

    fn register_action_handler(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.picker.register_action_handler(tx.clone())?;
        self.viewer.register_action_handler(tx)?;
        Ok(())
    }

    fn handle_events(&mut self, event: Event) -> Option<Action> {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }) = event
        {
            return Some(Action::Quit);
        };
        if let Event::Key(key_event) = event.clone() {
            self.last_event = key_event_to_string(&key_event);
        }
        match self.mode {
            Mode::Picker => self.picker.handle_events(event),
            Mode::Viewer(_) => self.viewer.handle_events(event),
            Mode::Help => self.help.handle_events(event),
            Mode::Waiting => None,
        }
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Init => self.init()?,
            Action::Quit => self.quit(),
            Action::Tick => self.tick().unwrap(),
            Action::Pause(ref m) => {
                self.previous_mode = m.clone();
                self.mode = Mode::Waiting;
            }
            Action::UnPause => self.mode = self.previous_mode.clone(),
            Action::SwitchModeToViewer(i) => {
                let d = self.picker.datasets.lock().unwrap()[i].clone();
                self.previous_mode = self.mode.clone();
                self.mode = Mode::Viewer(d.name.clone());
            }
            Action::SwitchModeToPicker => {
                self.previous_mode = self.mode.clone();
                self.mode = Mode::Picker;
            }
            Action::SwitchModeToHelp => {
                self.previous_mode = self.mode.clone();
                log::debug!("Previous mode = {:?}", self.previous_mode);
                self.mode = Mode::Help;
                self.help.previous_mode = self.previous_mode.clone();
                match self.previous_mode {
                    Mode::Picker => {
                        self.picker.focus = false;
                    }
                    Mode::Viewer(_) => {
                        self.viewer.focus = false;
                    }
                    _ => {}
                }
            }
            Action::SwitchModeToPreviousMode => {
                let last_mode = self.mode.clone();
                self.mode = self.previous_mode.clone();
                match self.mode {
                    Mode::Picker => {
                        self.picker.focus = true;
                    }
                    Mode::Viewer(_) => {
                        self.viewer.focus = true;
                    }
                    _ => {}
                }
                self.previous_mode = last_mode;
            }
            _ => (),
        };

        match self.mode {
            Mode::Picker => self.picker.update(action),
            Mode::Viewer(ref name) => {
                self.viewer.name.clone_from(name);
                self.viewer.file.clone_from(&self.file);
                self.viewer.update(action)
            }
            Mode::Help => self.help.update(action),
            _ => Ok(None),
        }
    }

    fn draw(&mut self, f: &mut Frame, rect: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(100),
                Constraint::Min(1),
                Constraint::Min(1),
            ])
            .split(rect);
        match self.mode {
            Mode::Picker => {
                self.picker.draw(f, chunks[0]);
            }
            Mode::Viewer(_) => {
                self.viewer.draw(f, chunks[0]);
            }
            Mode::Waiting => {}
            Mode::Help => {
                match self.previous_mode {
                    Mode::Picker => {
                        self.picker.draw(f, chunks[0]);
                    }
                    Mode::Viewer(_) => {
                        self.viewer.draw(f, chunks[0]);
                    }
                    _ => {}
                };
                self.help.draw(
                    f,
                    chunks[0].inner(&Margin {
                        vertical: 5,
                        horizontal: 5,
                    }),
                )
            }
        };
        let help_message = vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "q",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Gray),
            ),
            Span::styled(" to exit, ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "?",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Gray),
            ),
            Span::styled(" to view help, ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "◄ ▲ ▼ ►",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Gray),
            ),
            Span::styled(" to navigate.", Style::default().fg(Color::DarkGray)),
        ];
        let text = Text::from(Line::from(help_message));
        let help_message = Paragraph::new(text);
        f.render_widget(help_message, chunks[1]);

        let about_message = vec![
            Span::styled(
                "https://github.com/ECCC-RPE-EPR/e2020-data-viewer",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Gray),
            ),
            "#v".into(),
            Span::styled(
                env!("CARGO_PKG_VERSION"),
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Gray),
            ),
        ];
        let text = Text::from(Line::from(about_message));
        let about_message = Paragraph::new(text).alignment(Alignment::Right);
        f.render_widget(about_message, chunks[1]);
        if !self.last_event.is_empty() {
            f.render_widget(
                Block::default()
                    .title(
                        ratatui::widgets::block::Title::from(format!("{:?}", &self.last_event))
                            .alignment(Alignment::Right),
                    )
                    .title_style(Style::default().add_modifier(Modifier::BOLD)),
                Rect {
                    x: chunks[0].x + 1,
                    y: chunks[0].height.saturating_sub(1),
                    width: chunks[0].width.saturating_sub(2),
                    height: 1,
                },
            )
        }
    }
}
