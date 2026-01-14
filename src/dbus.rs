use gtk::{
    gio::{self, prelude::ApplicationExt},
    glib::{self},
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

pub struct StockfinBus {
    pub avg_change: Arc<Mutex<f64>>,
}

#[interface(name = "org.jlodenius.stockfin")]
impl StockfinBus {
    fn activate(&self) {
        glib::MainContext::default().invoke_local(|| {
            if let Some(app) = gio::Application::default() {
                app.activate();
            }
        });
    }

    #[zbus(property)]
    fn status_json(&self) -> String {
        let val = *self.avg_change.lock().unwrap();
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
            "text": format!("Stockfin: {}{:.2}%", sign, percentage),
            "alt": class,
            "class": class,
            "tooltip": format!("Portfolio Avg: {:.2}%", percentage)
        })
        .to_string()
    }
}

impl StockfinBus {
    pub fn spawn(avg_change: Arc<Mutex<f64>>) {
        let bus_interface = Self { avg_change };

        glib::MainContext::default().spawn_local(async move {
            let connection = Connection::session().await.expect("Failed to connect");

            // Only register the interface if we successfully get the name
            if (connection.request_name("org.jlodenius.stockfin").await).is_ok() {
                connection
                    .object_server()
                    .at("/org/jlodenius/stockfin", bus_interface)
                    .await
                    .expect("Failed to serve object");
                std::future::pending::<()>().await;
            }
        });
    }
}
