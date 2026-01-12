use crate::stock_object::StockObject;
use gtk::{
    ColumnView, ColumnViewColumn, Label, ScrolledWindow, SignalListItemFactory, SingleSelection,
    gio::{ListStore, prelude::ListModelExt},
    glib::{
        self,
        object::{Cast, CastNone, ObjectExt},
    },
    prelude::{ListItemExt, WidgetExt},
};

pub struct StockManager {
    pub model: ListStore,
}

impl StockManager {
    pub fn new(tickers: &[&'static str]) -> Self {
        let model = ListStore::new::<StockObject>();

        for ticker in tickers {
            model.append(&StockObject::new(ticker, 123.456));
        }

        Self { model }
    }

    pub fn update_prices(&self) {
        for i in 0..self.model.n_items() {
            if let Some(item) = self.model.item(i) {
                let stock = item.downcast::<StockObject>().unwrap();
                let new_price = stock.price() + 1.0;
                stock.set_price(new_price);
            }
        }
    }

    pub fn create_stock_list(&self) -> ScrolledWindow {
        let selection_model = SingleSelection::new(Some(self.model.clone()));
        let column_view = ColumnView::new(Some(selection_model));

        // --- Column 1: Ticker ---
        let factory_ticker = SignalListItemFactory::new();
        factory_ticker.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(gtk::Align::Start);
            list_item.set_child(Some(&label));
        });
        factory_ticker.connect_bind(|_, list_item| {
            let stock = list_item.item().and_downcast::<StockObject>().unwrap();
            let label = list_item.child().and_downcast::<Label>().unwrap();
            label.set_text(&stock.ticker());
        });
        let col_ticker = ColumnViewColumn::new(Some("Ticker"), Some(factory_ticker));
        column_view.append_column(&col_ticker);

        // --- Column 2: Price ---
        let factory_price = gtk::SignalListItemFactory::new();
        factory_price.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(gtk::Align::End);
            list_item.set_child(Some(&label));
        });
        factory_price.connect_bind(|_, list_item| {
            let stock = list_item.item().and_downcast::<StockObject>().unwrap();
            let label = list_item.child().and_downcast::<Label>().unwrap();

            // This creates a permanent link between the object property and the label text
            stock
                .bind_property("price", &label, "label")
                .transform_to(|_, value: f64| Some(format!("${:.2}", value)))
                .sync_create()
                .build();

            stock.connect_notify_local(
                Some("price"),
                glib::clone!(
                    #[weak]
                    label,
                    move |s, _| {
                        let price = s.property::<f64>("price");

                        if price > 200.0 {
                            label.add_css_class("success");
                            label.remove_css_class("error");
                        } else {
                            label.add_css_class("error");
                            label.remove_css_class("success");
                        }
                    }
                ),
            );
        });

        let col_price = ColumnViewColumn::new(Some("Price"), Some(factory_price));
        column_view.append_column(&col_price);

        ScrolledWindow::builder().child(&column_view).build()
    }
}
