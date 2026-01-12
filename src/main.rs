pub mod stock_api;
pub mod stock_manager;
pub mod stock_object;

use crate::stock_manager::StockManager;
use gtk::{
    gio::{self, ActionEntry, MenuItem},
    glib::{ControlFlow, timeout_add_local},
    prelude::*,
};
use std::{rc::Rc, time::Duration};

#[tokio::main]
async fn main() {
    let application = gtk::Application::builder()
        .application_id("org.stockfin")
        .build();

    application.connect_startup(on_startup);
    application.connect_activate(on_activate);
    application.run();
}

fn on_startup(app: &gtk::Application) {
    let about = ActionEntry::builder("about")
        .activate(|_, _, _| println!("About was pressed"))
        .build();

    let quit = ActionEntry::builder("quit")
        .activate(|app: &gtk::Application, _, _| app.quit())
        .build();

    app.add_action_entries([about, quit]);

    let menubar = {
        let file_menu = {
            let about_menu_item = MenuItem::new(Some("About"), Some("app.about"));
            let quit_menu_item = MenuItem::new(Some("Quit"), Some("app.quit"));

            let file_menu = gio::Menu::new();
            file_menu.append_item(&about_menu_item);
            file_menu.append_item(&quit_menu_item);
            file_menu
        };

        let menubar = gio::Menu::new();
        menubar.append_submenu(Some("File"), &file_menu);

        menubar
    };

    app.set_menubar(Some(&menubar));
}

fn on_activate(application: &gtk::Application) {
    let stock_manager = Rc::new(StockManager::new(&["GOOGL", "LUG.ST", "EQIX", "AAPL"]));

    // Update prices once every 10 seconds
    let manager_clone = stock_manager.clone();
    timeout_add_local(Duration::from_secs(10), move || {
        manager_clone.update_stocks();

        // Continue = keep timer running
        ControlFlow::Continue
    });

    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Stockfin")
        .default_width(400)
        .default_height(400)
        .child(&stock_manager.create_stock_list())
        .show_menubar(true)
        .build();

    window.present();
}
