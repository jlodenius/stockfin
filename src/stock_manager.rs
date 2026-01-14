use crate::{
    dbus::StockfinBusState,
    persistence::save_tickers,
    stock_api::{RangeResponse, StockApi},
    stock_object::StockObject,
};
use gtk::{
    Align, Box, ColumnView, ColumnViewColumn, CustomSorter, GestureClick, INVALID_LIST_POSITION,
    Label, ListBox, ListBoxRow, Orientation, Popover, PopoverMenu, PopoverMenuFlags, PositionType,
    ScrolledWindow, SearchEntry, SignalListItemFactory, SingleSelection, SortListModel,
    SorterChange,
    gdk::Rectangle,
    gio::{ListStore, Menu, SimpleAction, SimpleActionGroup, prelude::*},
    glib::{
        self,
        object::{Cast, CastNone, ObjectExt},
    },
    pango::EllipsizeMode,
    prelude::*,
};
use std::{cmp::Ordering, rc::Rc, sync::Arc};

pub struct StockManager {
    api: Rc<StockApi>,
    stocks: ListStore,
    sorted_stocks: SortListModel,
    bus_state: Arc<StockfinBusState>,
}

impl StockManager {
    pub fn new(tickers: &[(String, String)], bus_state: Arc<StockfinBusState>) -> Self {
        let sorter = CustomSorter::new(move |a, b| {
            let stock1 = a.downcast_ref::<StockObject>().unwrap();
            let stock2 = b.downcast_ref::<StockObject>().unwrap();

            stock2
                .pct_change_1d()
                .partial_cmp(&stock1.pct_change_1d())
                .unwrap_or(Ordering::Equal)
                .into()
        });

        let api = Rc::new(StockApi::new());
        let stocks = ListStore::new::<StockObject>();
        let sorted_stocks = SortListModel::new(Some(stocks.clone()), Some(sorter));

        for (ticker, name) in tickers {
            stocks.append(&StockObject::new(ticker, name));
        }

        let manager = Self {
            api,
            stocks,
            sorted_stocks,
            bus_state,
        };

        manager.update_stocks();
        manager
    }

    pub fn update_stocks(&self) {
        for i in 0..self.stocks.n_items() {
            if let Some(item) = self.stocks.item(i) {
                let stock = item.downcast::<StockObject>().unwrap();
                let sorted_stocks = self.sorted_stocks.clone();
                let ticker = stock.ticker();

                glib::MainContext::default().spawn_local({
                    let stock = stock.clone();
                    let api = self.api.clone();
                    let bus_state = self.bus_state.clone();

                    async move {
                        if let Ok(RangeResponse {
                            last_close,
                            pct_change,
                            ..
                        }) = api.weekly_range(&ticker).await
                        {
                            stock.set_pct_change_1w(pct_change);
                            stock.set_price(last_close);
                        }

                        if let Ok(RangeResponse {
                            pct_change,
                            last_close,
                            ..
                        }) = api.daily_range(&ticker).await
                        {
                            stock.set_pct_change_1d(pct_change);
                            stock.set_price(last_close);
                            *bus_state.avg_change.lock().unwrap() = pct_change;
                        }

                        // Makes sure that the UI updates
                        if let Some(sorter) = sorted_stocks.sorter() {
                            sorter.changed(SorterChange::Different);
                        }
                    }
                });
            }
        }
    }

    pub fn create_stock_list(&self) -> ScrolledWindow {
        let selection_model = SingleSelection::new(Some(self.sorted_stocks.clone()));
        let column_view = ColumnView::new(Some(selection_model));
        column_view.set_reorderable(false);

        // --- Action Setup ---
        let action_group = SimpleActionGroup::new();
        let remove_stock_action = SimpleAction::new("remove", None);

        remove_stock_action.connect_activate(glib::clone!(
            #[weak(rename_to = stocks)]
            self.stocks,
            #[weak(rename_to = sorted_stocks)]
            self.sorted_stocks,
            #[weak]
            column_view,
            move |_, _| {
                let selection = column_view
                    .model()
                    .and_downcast::<SingleSelection>()
                    .unwrap();
                let view_pos = selection.selected();

                if view_pos != INVALID_LIST_POSITION {
                    // Find the item in the sorted view
                    if let Some(item) = sorted_stocks.item(view_pos) {
                        // Find where this specific object lives in the underlying store
                        let mut source_pos = None;
                        for i in 0..stocks.n_items() {
                            if stocks.item(i).as_ref() == Some(&item) {
                                source_pos = Some(i);
                                break;
                            }
                        }

                        // Remove it from the store
                        if let Some(pos) = source_pos {
                            stocks.remove(pos);

                            let tickers: Vec<(String, String)> = (0..stocks.n_items())
                                .filter_map(|i| stocks.item(i))
                                .map(|obj| {
                                    let stock = obj.downcast::<StockObject>().unwrap();
                                    (stock.ticker(), stock.name())
                                })
                                .collect();

                            save_tickers(tickers);
                        }
                    }
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
                popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 0, 0)));
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
                .transform_to(|_, value: f64| Some(format!("{:.2}", value)))
                .sync_create()
                .build();
        });
        let col_price = ColumnViewColumn::new(Some("Price"), Some(factory_price));
        column_view.append_column(&col_price);

        // --- Column 4: 1W Change ---
        let factory_change_1w = SignalListItemFactory::new();
        factory_change_1w.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(Align::End);
            list_item.set_child(Some(&label));
        });
        factory_change_1w.connect_bind(|_, list_item| {
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
        let col_change_1w = ColumnViewColumn::new(Some("Change (1w)"), Some(factory_change_1w));
        column_view.append_column(&col_change_1w);

        // --- Column 4: 1d Change ---
        let factory_change_1d = SignalListItemFactory::new();
        factory_change_1d.connect_setup(|_, list_item| {
            let label = Label::new(None);
            label.set_halign(Align::End);
            list_item.set_child(Some(&label));
        });
        factory_change_1d.connect_bind(|_, list_item| {
            let stock = list_item.item().and_downcast::<StockObject>().unwrap();
            let label = list_item.child().and_downcast::<Label>().unwrap();

            stock
                .bind_property("pct-change-1d", &label, "label")
                .transform_to(|_, val: f64| {
                    let sign = if val >= 0.0 { "+" } else { "" };
                    Some(format!("{}{:.2}%", sign, val * 100.0))
                })
                .sync_create()
                .build();

            stock.connect_notify_local(
                Some("pct-change-1d"),
                glib::clone!(
                    #[weak]
                    label,
                    move |s, _| {
                        let pct_change = s.property::<f64>("pct-change-1d");
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
        let col_change_1d = ColumnViewColumn::new(Some("Change (1d)"), Some(factory_change_1d));
        column_view.append_column(&col_change_1d);

        ScrolledWindow::builder().child(&column_view).build()
    }

    pub fn create_search_bar(&self) -> Box {
        let container = Box::new(Orientation::Vertical, 6);
        let search_entry = SearchEntry::builder()
            .placeholder_text("Search ticker and press Enter...")
            .margin_top(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let results_popover = Popover::builder()
            .autohide(true)
            .position(PositionType::Bottom)
            .build();
        results_popover.set_parent(&search_entry);

        let results_list = ListBox::new();
        results_popover.set_child(Some(&results_list));

        // --- TRIGGER SEARCH ON ENTER ---
        search_entry.connect_activate(glib::clone!(
            #[weak(rename_to = api)]
            self.api,
            #[weak]
            results_popover,
            #[weak]
            results_list,
            move |entry| {
                let text = entry.text().to_string();
                if text.is_empty() {
                    return;
                }

                glib::MainContext::default().spawn_local(glib::clone!(
                    #[weak]
                    api,
                    #[weak]
                    results_popover,
                    #[weak]
                    results_list,
                    async move {
                        let results = api.search_ticker(&text).await;

                        // Clear old results
                        while let Some(child) = results_list.first_child() {
                            results_list.remove(&child);
                        }

                        if results.is_empty() {
                            results_popover.popdown();
                            return;
                        }

                        for (symbol, name) in results {
                            let symbol = glib::markup_escape_text(&symbol);
                            let name = glib::markup_escape_text(&name);

                            let label = Label::builder()
                                .label(format!("<b>{}</b> - {}", symbol, name))
                                .use_markup(true)
                                .xalign(0.0)
                                .build();

                            let row = ListBoxRow::new();
                            row.set_child(Some(&label));

                            unsafe {
                                row.set_data("ticker_symbol", symbol.to_string());
                                row.set_data("stock_name", name.to_string());
                            }

                            results_list.append(&row);
                        }
                        results_popover.popup();
                    }
                ));
            }
        ));

        // --- SELECTION LOGIC ---
        results_list.connect_row_activated(glib::clone!(
            #[weak(rename_to = model)]
            self.stocks,
            #[weak]
            results_popover,
            #[weak]
            search_entry,
            move |_, row| {
                // SAFETY:
                // We set these exact two values above
                let symbol: String = unsafe {
                    row.data::<String>("ticker_symbol")
                        .map(|s| s.as_ref().clone())
                        .unwrap_or_default()
                };
                let stock_name: String = unsafe {
                    row.data::<String>("stock_name")
                        .map(|s| s.as_ref().clone())
                        .unwrap_or_default()
                };

                if !symbol.is_empty() {
                    model.append(&StockObject::new(&symbol, &stock_name));

                    let tickers: Vec<(String, String)> = (0..model.n_items())
                        .filter_map(|i| model.item(i))
                        .map(|obj| {
                            let stock = obj.downcast::<StockObject>().unwrap();
                            (stock.ticker(), stock.name())
                        })
                        .collect();

                    save_tickers(tickers);
                }

                search_entry.set_text("");
                results_popover.popdown();
            }
        ));

        container.append(&search_entry);
        container
    }
}
