use crate::{
    stock_api::{StockApi, WeeklyRangeResponse},
    stock_object::StockObject,
};
use gtk::{
    Align, ColumnView, ColumnViewColumn, GestureClick, INVALID_LIST_POSITION, Label, PopoverMenu,
    PopoverMenuFlags, ScrolledWindow, SignalListItemFactory, SingleSelection,
    gio::{
        ListStore, Menu, SimpleAction, SimpleActionGroup,
        prelude::{ActionMapExt, ListModelExt},
    },
    glib::{
        self,
        object::{Cast, CastNone, ObjectExt},
    },
    pango::EllipsizeMode,
    prelude::{GestureSingleExt, ListItemExt, PopoverExt, WidgetExt},
};
use std::rc::Rc;

pub struct StockManager {
    api: Rc<StockApi>,
    model: ListStore,
}

impl StockManager {
    pub fn new(tickers: &[&'static str]) -> Self {
        let api = Rc::new(StockApi::new());
        let model = ListStore::new::<StockObject>();

        for ticker in tickers {
            model.append(&StockObject::new(ticker));
        }

        let manager = Self { api, model };
        manager.update_stocks();
        manager
    }

    pub fn update_stocks(&self) {
        for i in 0..self.model.n_items() {
            if let Some(item) = self.model.item(i) {
                let stock = item.downcast::<StockObject>().unwrap();
                let ticker = stock.ticker();

                glib::MainContext::default().spawn_local({
                    let stock = stock.clone();
                    let api = self.api.clone();

                    async move {
                        if let Ok(WeeklyRangeResponse {
                            stock_name,
                            prev_close,
                            last_close,
                        }) = api.weekly_range(&ticker).await
                        {
                            stock.set_name(stock_name);
                            stock.set_pct_change_1w((last_close - prev_close) / prev_close);
                            stock.set_price(last_close);
                        }
                    }
                });
            }
        }
    }

    pub fn create_stock_list(&self) -> ScrolledWindow {
        let selection_model = SingleSelection::new(Some(self.model.clone()));
        let column_view = ColumnView::new(Some(selection_model));
        column_view.set_reorderable(false);

        // --- Action Setup ---
        let action_group = SimpleActionGroup::new();
        let remove_stock_action = SimpleAction::new("remove", None);

        remove_stock_action.connect_activate(glib::clone!(
            #[weak(rename_to = model)]
            self.model,
            #[weak]
            column_view,
            move |_, _| {
                let selection = column_view
                    .model()
                    .and_downcast::<SingleSelection>()
                    .unwrap();
                let pos = selection.selected();
                if pos != INVALID_LIST_POSITION {
                    model.remove(pos);
                }
            }
        ));
        action_group.add_action(&remove_stock_action);
        column_view.insert_action_group("stock", Some(&action_group));

        // --- Menu UI Setup ---
        let menu_model = Menu::new();
        menu_model.append(Some("Remove"), Some("stock.remove"));
        let popover = PopoverMenu::from_model_full(&menu_model, PopoverMenuFlags::NESTED);
        popover.set_parent(&column_view);
        popover.set_has_arrow(false);

        // --- Right Click Gesture ---
        let gesture = GestureClick::new();
        gesture.set_button(3);
        gesture.connect_pressed(glib::clone!(
            #[weak]
            popover,
            move |_, _, x, y| {
                popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 0, 0)));
                popover.popup();
            }
        ));
        column_view.add_controller(gesture);

        // --- Column 1: Ticker ---
        let factory_ticker = SignalListItemFactory::new();
        factory_ticker.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(Align::Start);
            list_item.set_child(Some(&label));
        });
        factory_ticker.connect_bind(|_, list_item| {
            let stock = list_item.item().and_downcast::<StockObject>().unwrap();
            let label = list_item.child().and_downcast::<Label>().unwrap();
            label.set_text(&stock.ticker());
        });
        let col_ticker = ColumnViewColumn::new(Some("Ticker"), Some(factory_ticker));
        column_view.append_column(&col_ticker);

        // --- Column 2: Name ---
        let factory_name = SignalListItemFactory::new();
        factory_name.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(Align::Start);
            label.set_ellipsize(EllipsizeMode::End);
            list_item.set_child(Some(&label));
        });
        factory_name.connect_bind(|_, list_item| {
            let stock = list_item.item().and_downcast::<StockObject>().unwrap();
            let label = list_item.child().and_downcast::<Label>().unwrap();
            stock
                .bind_property("name", &label, "label")
                .sync_create()
                .build();
        });
        let col_name = ColumnViewColumn::new(Some("Stock"), Some(factory_name));
        col_name.set_expand(true);
        column_view.append_column(&col_name);

        // --- Column 3: Price ---
        let factory_price = SignalListItemFactory::new();
        factory_price.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(Align::End);
            list_item.set_child(Some(&label));
        });
        factory_price.connect_bind(|_, list_item| {
            let stock = list_item.item().and_downcast::<StockObject>().unwrap();
            let label = list_item.child().and_downcast::<Label>().unwrap();

            stock
                .bind_property("price", &label, "label")
                .transform_to(|_, value: f64| Some(format!("${:.2}", value)))
                .sync_create()
                .build();
        });
        let col_price = ColumnViewColumn::new(Some("Price"), Some(factory_price));
        column_view.append_column(&col_price);

        // --- Column 4: 1W Change ---
        let factory_change = SignalListItemFactory::new();
        factory_change.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(Align::End);
            list_item.set_child(Some(&label));
        });
        factory_change.connect_bind(|_, list_item| {
            let stock = list_item.item().and_downcast::<StockObject>().unwrap();
            let label = list_item.child().and_downcast::<Label>().unwrap();

            stock
                .bind_property("pct-change-1w", &label, "label")
                .transform_to(|_, val: f64| {
                    let sign = if val >= 0.0 { "+" } else { "" };
                    Some(format!("{}{:.2}%", sign, val * 100.0))
                })
                .sync_create()
                .build();

            stock.connect_notify_local(
                Some("pct-change-1w"),
                glib::clone!(
                    #[weak]
                    label,
                    move |s, _| {
                        let pct_change = s.property::<f64>("pct-change-1w");
                        if pct_change >= 0.0 {
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
        let col_change = ColumnViewColumn::new(Some("Change (1w)"), Some(factory_change));
        column_view.append_column(&col_change);

        ScrolledWindow::builder().child(&column_view).build()
    }
}
