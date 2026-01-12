use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use glib::Properties;
    use std::cell::RefCell;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::StockObject)]
    pub struct StockObject {
        #[property(get, set)]
        pub ticker: RefCell<String>,
        #[property(get, set)]
        pub price: RefCell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockObject {
        const NAME: &'static str = "StockObject";
        type Type = super::StockObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for StockObject {}
}

glib::wrapper! {
    pub struct StockObject(ObjectSubclass<imp::StockObject>);
}

impl StockObject {
    pub fn new(ticker: &str, price: f64) -> Self {
        glib::Object::builder()
            .property("ticker", ticker)
            .property("price", price)
            .build()
    }
}
