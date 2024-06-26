use std::io::Stderr;

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use ratatui_macros::{line, text};
use tui_input::Input;

use crate::{
    action::Action,
    components::{Component, Frame},
    data::Data,
    runner::Runner,
};

#[derive(Default, Debug)]
pub struct Summary {
    pub scroll: u16,
    pub name: String,
    pub doc: String,
    pub kvs: Vec<(String, String)>,
    pub kis: Vec<usize>,
    pub total_indices: Vec<usize>,
    pub axis0: usize,
    pub axis1: usize,
}

impl Summary {
    pub fn refresh(
        &mut self,
        d: Data,
        indices: Vec<usize>,
        axis0: usize,
        axis1: usize,
    ) -> Result<()> {
        self.kvs = Vec::default();
        self.kis = Vec::default();
        self.total_indices = Vec::default();
        self.name.clone_from(&d.name);
        self.doc.clone_from(&d.doc);
        self.axis0 = axis0;
        self.axis1 = axis1;
        for (i, dim) in d.set_names.iter().enumerate() {
            let set_data = d.set_data[i].clone();
            let shape = d.shape.clone();
            self.kvs.push((dim.clone(), set_data[indices[i]].clone()));
            self.kis.push(indices[i]);
            self.total_indices.push(set_data.len());
        }
        Ok(())
    }
}

impl Component for Summary {
    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) {
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title("Dataset")
                .border_style(Style::default().add_modifier(Modifier::DIM)),
            rect,
        );
        let [top_rect, bottom_rect] =
            Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(rect);

        let text = text![
            "",
            self.name.clone(),
            Span::styled(
                &self.doc,
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::DarkGray),
            ),
            self.kvs
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    if i == self.axis0 || i == self.axis1 {
                        Span::styled(format!(" {} ", k), Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw(format!(" {} ", k))
                    }
                })
                .collect::<Vec<Span>>(),
        ];
        f.render_widget(Paragraph::new(text).alignment(Alignment::Center), top_rect);

        let [left_rect, middle_left_rect, middle_right_rect, right_rect] = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Length(10),
            Constraint::Percentage(50),
        ])
        .areas(bottom_rect);
        let mut text_left = vec![];
        let mut text_middle_left = vec![];
        let mut text_middle_right = vec![];
        let mut text_right = vec![];
        for (i, (k, v)) in self.kvs.iter().enumerate() {
            let index = self.kis[i] + 1;
            let total_index = self.total_indices[i];
            if i == self.axis0 || i == self.axis1 {
                continue;
            }
            let i = i + 1;
            text_left.push(Line::from(vec![
                Span::styled(format!(" {k}"), Style::default().fg(Color::Yellow)),
                Span::raw(": "),
            ]));
            text_middle_left.push(Line::from(vec![Span::styled(
                v,
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            text_middle_right.push(Line::from(vec![Span::styled(
                format!(" ({index} / {total_index})"),
                Style::default().fg(Color::DarkGray),
            )]));
            text_right.push(Line::from(vec![
                Span::styled(" ↓ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("F{i}"),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Gray),
                ),
                Span::styled(" ↑ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("Shift + F{i}"),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Gray),
                ),
            ]));
        }

        f.render_widget(
            Paragraph::new(text_left).alignment(Alignment::Right),
            left_rect,
        );
        f.render_widget(
            Paragraph::new(text_middle_left).alignment(Alignment::Left),
            middle_left_rect,
        );
        f.render_widget(
            Paragraph::new(text_middle_right).alignment(Alignment::Right),
            middle_right_rect,
        );
        f.render_widget(
            Paragraph::new(text_right).alignment(Alignment::Left),
            right_rect,
        );
    }
}
