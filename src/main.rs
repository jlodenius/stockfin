pub mod dbus;
pub mod persistence;
pub mod stock_api;
pub mod stock_manager;
pub mod stock_object;

use crate::{dbus::StockfinBus, persistence::load_tickers, stock_manager::StockManager};
use gtk::{
    Application, ApplicationWindow, Box, CssProvider, Orientation,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
    gdk::Display,
    glib::{ControlFlow, Propagation, timeout_add_local},
    prelude::*,
    style_context_add_provider_for_display,
};
use std::{rc::Rc, time::Duration};

fn main() {
    let application = Application::builder()
        .application_id("org.jlodenius.stockfin")
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
    // If app already running, focus it instead of starting a new instance
    if let Some(window) = application.active_window() {
        return window.present();
    }

    let tickers = load_tickers();
    let bus_state = StockfinBus::spawn();
    let stock_manager = Rc::new(StockManager::new(&tickers, bus_state));

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

    window.connect_close_request(move |w| {
        w.hide();
        Propagation::Stop // Prevent window from being destroyed
    });
    window.present();

    // Update prices once every 60 seconds
    let manager_clone = stock_manager.clone();
    timeout_add_local(Duration::from_secs(60), move || {
        manager_clone.update_stocks();

        // Continue = keep timer running
        ControlFlow::Continue
    });
}
