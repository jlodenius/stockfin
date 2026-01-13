pub mod persistence;
pub mod stock_api;
pub mod stock_manager;
pub mod stock_object;

use crate::{persistence::load_tickers, stock_manager::StockManager};
use gtk::{
    Application, ApplicationWindow, Box, CssProvider, Orientation,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
    gdk::Display,
    glib::{ControlFlow, timeout_add_local},
    prelude::*,
    style_context_add_provider_for_display,
};
use std::{rc::Rc, time::Duration};

#[tokio::main]
async fn main() {
    let application = Application::builder()
        .application_id("org.stockfin")
        .build();

    application.connect_startup(on_startup);
    application.connect_activate(on_activate);
    application.run();
}

fn on_startup(_app: &Application) {
    let css_provider = CssProvider::new();
    css_provider.load_from_path("resources/style.css");

    style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &css_provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn on_activate(application: &Application) {
    let tickers = load_tickers();
    let stock_manager = Rc::new(StockManager::new(&tickers));

    let main_layout = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .build();

    let stock_list = stock_manager.create_stock_list();
    stock_list.set_vexpand(true);
    main_layout.append(&stock_manager.create_search_bar());
    main_layout.append(&stock_list);

    let window = ApplicationWindow::builder()
        .application(application)
        .title("Stockfin")
        .default_width(400)
        .default_height(400)
        .child(&main_layout)
        .show_menubar(true)
        .build();

    window.present();

    // Update prices once every 10 seconds
    let manager_clone = stock_manager.clone();
    timeout_add_local(Duration::from_secs(10), move || {
        manager_clone.update_stocks();

        // Continue = keep timer running
        ControlFlow::Continue
    });
}
