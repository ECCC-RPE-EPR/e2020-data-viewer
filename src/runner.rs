use std::sync::Arc;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    action::Action,
    components::{app::App, Component},
    data::Data,
    trace_dbg, tui,
    tui::Event,
};

#[derive(Default)]
pub struct Runner {
    pub tick_rate: f64,
    pub frame_rate: f64,
    pub components: Vec<Box<dyn Component>>,
    pub should_quit: bool,
    pub should_suspend: bool,
}

impl Runner {
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        file: String,
        dataset: Option<String>,
    ) -> Result<Self> {
        let app = App::new(file, dataset)?;
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![Box::new(app)],
            should_quit: false,
            should_suspend: false,
        })
    }

    pub fn quit(&mut self) {
        self.should_quit = true
    }

    pub async fn run(&mut self) -> Result<()> {
        let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut tui = tui::Tui::new()?;
        tui.tick_rate(self.tick_rate);
        tui.frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(action_tx.clone())?;
        }

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    tui::Event::Init => action_tx.send(Action::Init)?,
                    tui::Event::Quit => action_tx.send(Action::Quit)?,
                    tui::Event::Tick => action_tx.send(Action::Tick)?,
                    tui::Event::Render => action_tx.send(Action::Render)?,
                    tui::Event::Resize(x, y) => action_tx.send(Action::Resize { x, y })?,
                    e => {
                        for component in self.components.iter_mut() {
                            if let Some(action) = component.handle_events(e.clone()) {
                                action_tx.send(action)?;
                            }
                        }
                    }
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                if action != Action::Tick && action != Action::Render {
                    log::debug!("{action:?}");
                }
                match action {
                    Action::Quit => self.should_quit = true,
                    Action::Suspend => self.should_suspend = true,
                    Action::Resume => self.should_suspend = false,
                    Action::Render => {
                        tui.draw(|f| {
                            for component in self.components.iter_mut() {
                                component.draw(f, f.size());
                            }
                        })?;
                    }
                    _ => {}
                }
                for component in self.components.iter_mut() {
                    if let Some(action) = component.update(action.clone())? {
                        action_tx.send(action)?
                    };
                }
            }
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                tui = tui::Tui::new()?;
                tui.tick_rate(self.tick_rate);
                tui.frame_rate(self.frame_rate);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }
}
