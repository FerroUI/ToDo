#![allow(dead_code)]
use std::rc::Rc;
use todo_domain::{AppState, Date};
use todo_state::AppViewModel;

pub fn today() -> Date {
    "2026-11-21".parse().unwrap()
}
pub fn vm() -> Rc<AppViewModel> {
    AppViewModel::new(AppState::new(), today(), Rc::new(|_| Ok(())))
}
