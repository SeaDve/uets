use std::collections::HashSet;

use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::{map::Entry, IndexMap};

use crate::{stock::Stock, stock_id::StockId};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct StockList {
        pub(super) list: RefCell<IndexMap<StockId, Stock>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockList {
        const NAME: &'static str = "UetsStockList";
        type Type = super::StockList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for StockList {}

    impl ListModelImpl for StockList {
        fn item_type(&self) -> glib::Type {
            Stock::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, v)| v.upcast_ref::<glib::Object>())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub struct StockList(ObjectSubclass<imp::StockList>)
        @implements gio::ListModel;
}

impl StockList {
    pub fn from_raw(value: IndexMap<StockId, Stock>) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.list.replace(value);

        this
    }

    pub fn len(&self) -> usize {
        self.imp().list.borrow().len()
    }

    pub fn contains(&self, id: &StockId) -> bool {
        self.imp().list.borrow().contains_key(id)
    }

    pub fn get(&self, id: &StockId) -> Option<Stock> {
        self.imp().list.borrow().get(id).cloned()
    }

    pub fn insert(&self, stock: Stock) {
        let imp = self.imp();

        let (index, removed, added) = match imp.list.borrow_mut().entry(stock.id().clone()) {
            Entry::Occupied(entry) => (entry.index(), 1, 1),
            Entry::Vacant(entry) => {
                let index = entry.index();
                entry.insert(stock);
                (index, 0, 1)
            }
        };

        self.items_changed(index as u32, removed, added);
    }

    pub fn insert_many(&self, stock: Vec<Stock>) -> u32 {
        let mut updated_indices = HashSet::new();
        let mut n_appended = 0;

        {
            let mut list = self.imp().list.borrow_mut();

            for stock in stock {
                let (index, prev_value) = list.insert_full(stock.id().clone(), stock);

                if prev_value.is_some() {
                    updated_indices.insert(index);
                } else {
                    n_appended += 1;
                }
            }
        }

        let index_of_first_append = self.n_items() - n_appended;

        // Emit about the appended items first, so GTK would know about
        // the new items and it won't error out because the n_items
        // does not match what GTK expect
        if n_appended != 0 {
            self.items_changed(index_of_first_append, 0, n_appended);
        }

        // This is emitted individually because each updated item
        // may be on different indices
        for index in updated_indices {
            // Only emit if the updated item is before the first appended item
            // because it is already handled by the emission above
            if (index as u32) < index_of_first_append {
                self.items_changed(index as u32, 1, 1);
            }
        }

        n_appended
    }

    pub fn clear(&self) {
        let imp = self.imp();

        let prev_len = imp.list.borrow().len();

        if prev_len == 0 {
            return;
        }

        imp.list.borrow_mut().clear();
        self.items_changed(0, prev_len as u32, 0);
    }
}
