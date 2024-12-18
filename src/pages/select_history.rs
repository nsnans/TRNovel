use super::{network_novel::book_detail::BookDetail, Page, PageWrapper};
use crate::{
    app::State,
    components::{Component, Confirm, ConfirmState, Empty, KeyShortcutInfo},
    history::{History, HistoryItem},
    novel::{local_novel::LocalNovel, network_novel::NetworkNovel, NovelChapters},
    pages::ReadNovel,
    Navigator, NetworkNovelCache, Result, RoutePage, Router,
};
use anyhow::anyhow;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use parse_book_source::JsonSource;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Padding, Paragraph, Scrollbar, ScrollbarState, Widget},
};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Mutex};
use tui_widget_list::{ListBuilder, ListState, ListView};

struct ListItem {
    pub history: HistoryItem,
    pub selected: bool,
}

impl Widget for ListItem {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = if self.selected {
            Block::bordered()
                .padding(Padding::horizontal(0))
                .light_cyan()
        } else {
            Block::bordered().padding(Padding::horizontal(0))
        };

        let [top, bottom] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(block.inner(area));
        block.render(area, buf);

        let [bottom_left, bottom_right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(bottom);

        match self.history {
            HistoryItem::Local(item) => {
                Paragraph::new(Text::from(vec![
                    Line::from(item.title.clone()),
                    Line::from(item.current_chapter.clone()).centered(),
                ]))
                .render(top, buf);

                Span::from("本地小说").dim().render(bottom_left, buf);

                Text::from(
                    format!(
                        "{:.2}% {}",
                        item.percent,
                        item.last_read_at.format("%Y-%m-%d %H:%M:%S")
                    )
                    .dim(),
                )
                .right_aligned()
                .render(bottom_right, buf);
            }
            HistoryItem::Network(item) => {
                Paragraph::new(Text::from(vec![
                    Line::from(item.title.clone()),
                    Line::from(item.current_chapter.clone()).centered(),
                ]))
                .render(top, buf);

                Span::from(format!("书源：{}", item.book_source))
                    .dim()
                    .render(bottom_left, buf);

                Text::from(
                    format!(
                        "{:.2}% {}",
                        item.percent,
                        item.last_read_at.format("%Y-%m-%d %H:%M:%S")
                    )
                    .dim(),
                )
                .right_aligned()
                .render(bottom_right, buf);
            }
        };
    }
}

#[derive(Debug, Clone)]
pub struct SelectHistory {
    pub state: ListState,
    pub confirm_state: ConfirmState,
    pub history: Arc<Mutex<History>>,
    pub navigator: Navigator,
}

impl SelectHistory {
    pub fn new(history: Arc<Mutex<History>>, navigator: Navigator) -> Self {
        Self {
            history,
            state: ListState::default(),
            confirm_state: ConfirmState::default(),
            navigator,
        }
    }

    pub fn to_page_route() -> Box<dyn RoutePage> {
        let page: PageWrapper<Self, (), ()> = PageWrapper::new((), None);
        Box::new(page)
    }

    fn render_list(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let list_items = self.history.try_lock().unwrap().histories.clone();
        let length = list_items.len();
        let builder = ListBuilder::new(move |context| {
            let (_, item) = &list_items[context.index];
            (
                ListItem {
                    history: item.clone(),
                    selected: context.is_selected,
                },
                5,
            )
        });
        let widget = ListView::new(builder, length).infinite_scrolling(false);
        frame.render_stateful_widget(widget, area, &mut self.state);
    }
}

#[async_trait]
impl Component for SelectHistory {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) -> Result<()> {
        let block = Block::bordered()
            .title(Line::from("历史记录").centered())
            .border_style(Style::new().dim());

        let container_area = block.inner(area);

        if self.history.try_lock().unwrap().histories.is_empty() {
            frame.render_widget(Empty::new("暂无历史记录"), container_area);
            return Ok(());
        }

        self.render_list(frame, container_area);

        let len = self.history.try_lock()?.histories.len();
        let current = self.state.selected.unwrap_or(0);

        frame.render_widget(
            block.title_bottom(format!(" {}/{}", current + 1, len)),
            area,
        );

        if len * 5 > container_area.height as usize {
            let mut scrollbar_state = ScrollbarState::new(len).position(current);
            frame.render_stateful_widget(Scrollbar::default(), area, &mut scrollbar_state);
        }

        frame.render_stateful_widget(
            Confirm::new("警告", "确认删除该历史记录吗？"),
            area,
            &mut self.confirm_state,
        );
        Ok(())
    }

    async fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
        state: State,
    ) -> Result<Option<KeyEvent>> {
        if key.kind != KeyEventKind::Press {
            return Ok(Some(key));
        }
        if self.confirm_state.show {
            match key.code {
                KeyCode::Char('y') => {
                    self.confirm_state.confirm();
                    Ok(None)
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
                    self.confirm_state.toggle();
                    Ok(None)
                }
                KeyCode::Enter => {
                    if let Some(index) = self.state.selected {
                        if self.confirm_state.is_confirm() {
                            self.history.lock().await.remove_index(index);
                            self.history.lock().await.save()?;
                            self.state.select(None);
                        }
                    }
                    self.confirm_state.hide();
                    Ok(None)
                }
                KeyCode::Char('n') => {
                    self.confirm_state.hide();
                    Ok(None)
                }
                _ => Ok(Some(key)),
            }
        } else {
            match key.code {
                KeyCode::Char('h') | KeyCode::Left => {
                    self.state.select(None);
                    Ok(None)
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.state.next();
                    Ok(None)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.state.previous();
                    Ok(None)
                }
                KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                    let Some(index) = self.state.selected else {
                        return Err("请选择历史记录".into());
                    };
                    let (path, item) = &self.history.lock().await.histories[index];

                    match item {
                        HistoryItem::Local(_) => {
                            let novel = LocalNovel::from_path(path)?;
                            self.navigator
                                .push(Box::new(ReadNovel::to_page_route(novel)))?;
                        }
                        HistoryItem::Network(_) => {
                            let network_cache = NetworkNovelCache::try_from(path.as_str())?;
                            let json_source = state
                                .book_sources
                                .lock()
                                .await
                                .find_book_source(
                                    &network_cache.book_source_url,
                                    &network_cache.book_source_name,
                                )
                                .map(|i| i.clone())
                                .ok_or(anyhow!("book source not found"))?;

                            let novel = NetworkNovel {
                                book_list_item: network_cache.book_list_item,
                                book_source: Arc::new(Mutex::new(JsonSource::try_from(
                                    json_source,
                                )?)),
                                book_info: None,
                                novel_chapters: NovelChapters {
                                    current_chapter: network_cache.current_chapter,
                                    line_percent: network_cache.line_percent,
                                    chapters: None,
                                },
                            };

                            self.navigator.push(BookDetail::to_page_route(novel))?;
                        }
                    }

                    Ok(None)
                }
                KeyCode::Char('d') => {
                    if self.state.selected.is_none() {
                        return Err("请选择历史记录".into());
                    }

                    self.confirm_state.show();
                    Ok(None)
                }
                _ => Ok(Some(key)),
            }
        }
    }

    fn key_shortcut_info(&self) -> crate::components::KeyShortcutInfo {
        let data = if self.confirm_state.show {
            vec![
                ("确认删除", "Y"),
                ("取消删除", "N"),
                ("切换确定/取消", "H / ◄ / L / ► "),
                ("确认选中", "Enter"),
                ("切换到选择文件", "Tab"),
            ]
        } else {
            vec![
                ("选择下一个", "J / ▼"),
                ("选择上一个", "K / ▲"),
                ("取消选择", "H / ◄"),
                ("确认选择", "L / ► / Enter"),
                ("删除选中的历史记录", "D"),
                ("切换到选择文件", "Tab"),
            ]
        };
        KeyShortcutInfo::new(data)
    }
}

#[async_trait]
impl Page for SelectHistory {
    type Msg = ();

    async fn init(
        _arg: (),
        _sender: Sender<Self::Msg>,
        navigator: Navigator,
        state: State,
    ) -> Result<Self> {
        Ok(Self::new(state.history.clone(), navigator))
    }
}

impl Router for SelectHistory {}
