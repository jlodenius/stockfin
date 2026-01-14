use gtk::{
    Application,
    gio::{self, prelude::ApplicationExt},
    glib::{self, object::Cast},
    prelude::*,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use zbus::{connection::Connection, interface, proxy};

#[proxy(
    interface = "org.jlodenius.stockfin",
    default_service = "org.jlodenius.stockfin",
    default_path = "/org/jlodenius/stockfin"
)]
trait Stockfin {
    fn activate(&self) -> zbus::Result<()>;
}

pub struct StockfinBusState {
    pub avg_change: Arc<Mutex<f64>>,
}

impl StockfinBusState {
    fn new() -> Self {
        Self {
            avg_change: Arc::new(Mutex::new(0.0)),
        }
    }
}

pub struct StockfinBus {
    state: Arc<StockfinBusState>,
}

#[interface(name = "org.jlodenius.stockfin")]
impl StockfinBus {
    fn activate(&self) {
        // .invoke() safely moves the closure to the MAIN thread
        glib::MainContext::default().invoke(move || {
            let app =
                gio::Application::default().and_then(|app| app.downcast::<Application>().ok());

            if let Some(app) = app {
                match app.active_window() {
                    Some(window) => window.present(),
                    None => app.activate(),
                }
            }
        });
    }

    #[zbus(property)]
    fn status_json(&self) -> String {
        let val = *self.state.avg_change.lock().unwrap();
        let percentage = val * 100.0;
        let class = if percentage > 0.0 {
            "bullish"
        } else if percentage < 0.0 {
            "bearish"
        } else {
            "neutral"
        };
        let sign = if percentage >= 0.0 { "+" } else { "" };

        json!({
            "text": format!("{}{:.2}%", sign, percentage),
            "alt": class,
            "class": class,
            "tooltip": format!("Daily average: {:.2}%", percentage)
        })
        .to_string()
    }
}

impl StockfinBus {
    pub fn spawn() -> Arc<StockfinBusState> {
        let state = Arc::new(StockfinBusState::new());
        let bus = Self {
            state: state.clone(),
        };

        glib::MainContext::default().spawn_local(async move {
            let connection = Connection::session().await.expect("Failed to connect");

            connection
                .request_name("org.jlodenius.stockfin.Waybar")
                .await
                .expect("Failed to request name");

            connection
                .object_server()
                .at("/org/jlodenius/stockfin", bus)
                .await
                .expect("Failed to serve object");

            std::future::pending::<()>().await;
        });

        state
    }
}
