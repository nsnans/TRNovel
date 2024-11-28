use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, Frame};
use state::State;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
pub mod state;

use crate::{
    components::{warning::Warning, Component},
    events::{event_loop, Events},
    history::History,
    routes::{Route, Routes},
};

pub struct App {
    pub state: State,
    pub routes: Routes,
    pub show_exit: bool,
    pub event_rx: UnboundedReceiver<Events>,
    pub event_tx: UnboundedSender<Events>,
    pub warning: Option<String>,
    pub cancellation_token: CancellationToken,
}

impl App {
    pub fn new(path: PathBuf) -> Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();

        event_loop(tx.clone(), cancellation_token.clone());
        tx.send(Events::PushRoute(Route::SelectNovel(path)))?;
        let state = State {
            history: Arc::new(Mutex::new(History::load()?)),
        };
        Ok(Self {
            event_tx: tx.clone(),
            event_rx: rx,
            show_exit: false,
            warning: None,
            routes: Routes::new(tx, state.clone()),
            state,
            cancellation_token,
        })
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.show_exit {
            match self.handle_events(&mut terminal).await {
                Ok(_) => {}
                Err(e) => {
                    self.warning = Some(e.to_string());
                }
            }
        }
        Ok(())
    }

    pub async fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let Some(event) = self.event_rx.recv().await else {
            return Ok(());
        };

        if self.warning.is_none() {
            self.routes
                .handle_events(event.clone(), self.event_tx.clone(), self.state.clone())?;
        }

        match event {
            Events::KeyEvent(key) => {
                if key.kind == KeyEventKind::Press {
                    if self.warning.is_some() {
                        if key.code == KeyCode::Esc {
                            self.warning = None
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => {
                                self.cancellation_token.cancel();
                                self.show_exit = true;
                            }
                            KeyCode::Char('c') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    self.cancellation_token.cancel();
                                    self.show_exit = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Events::Render => {
                terminal.draw(|frame| match self.draw(frame) {
                    Ok(_) => {}
                    Err(e) => {
                        self.warning = Some(e.to_string());
                    }
                })?;
            }
            Events::Error(e) => self.warning = Some(e),
            _ => {}
        }

        Ok(())
    }

    pub fn draw(&mut self, frame: &mut Frame<'_>) -> anyhow::Result<()> {
        self.routes.draw(frame, frame.area())?;

        if let Some(warning) = &self.warning {
            frame.render_widget(Warning::new(warning), frame.area());
        }
        Ok(())
    }
}
