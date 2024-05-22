use std::io::Stderr;

use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{backend::CrosstermBackend, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    runner::Runner,
    tui::{Event, Frame},
};

pub mod app;
pub mod help;
pub mod picker;
pub mod select;
pub mod summary;
pub mod viewer;

pub trait Component {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        Ok(())
    }
    fn handle_events(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Key(key_event) => self.handle_key_events(key_event),
            Event::Mouse(mouse_event) => self.handle_mouse_events(mouse_event),
            _ => None,
        }
    }
    fn handle_key_events(&mut self, key: KeyEvent) -> Option<Action> {
        None
    }
    fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Option<Action> {
        None
    }
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        Ok(None)
    }
    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect);
}
