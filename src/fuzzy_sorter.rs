use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct FuzzySorter {
        pub(super) search: RefCell<String>,
        pub(super) default_sorter: RefCell<Option<gtk::Sorter>>,

        pub(super) fuzzy_matcher: OnceCell<Rc<SkimMatcherV2>>,
        pub(super) obj_choice_getter: OnceCell<Box<dyn Fn(&glib::Object) -> String>>,

        pub(super) default_sorter_changed_id: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FuzzySorter {
        const NAME: &'static str = "UetsFuzzySorter";
        type Type = super::FuzzySorter;
        type ParentType = gtk::Sorter;
    }

    impl ObjectImpl for FuzzySorter {}

    impl SorterImpl for FuzzySorter {
        fn compare(&self, obj_1: &glib::Object, obj_2: &glib::Object) -> gtk::Ordering {
            let search = self.search.borrow();

            if search.is_empty() {
                if let Some(default_sorter) = self.default_sorter.borrow().as_ref() {
                    default_sorter.compare(obj_1, obj_2)
                } else {
                    gtk::Ordering::Equal
                }
            } else {
                let choice_getter = self.obj_choice_getter.get().unwrap();
                let choice_1 = choice_getter(obj_1);
                let choice_2 = choice_getter(obj_2);

                let fuzzy_matcher = self.fuzzy_matcher.get().unwrap();
                let score_1 = fuzzy_matcher.fuzzy_match(&choice_1, &search);
                let score_2 = fuzzy_matcher.fuzzy_match(&choice_2, &search);
                score_2.cmp(&score_1).into()
            }
        }

        fn order(&self) -> gtk::SorterOrder {
            if self.search.borrow().is_empty() {
                if let Some(default_sorter) = self.default_sorter.borrow().as_ref() {
                    default_sorter.order()
                } else {
                    gtk::SorterOrder::None
                }
            } else {
                gtk::SorterOrder::Partial
            }
        }
    }
}

glib::wrapper! {
    pub struct FuzzySorter(ObjectSubclass<imp::FuzzySorter>)
        @extends gtk::Sorter;

}

impl FuzzySorter {
    pub fn new(
        fuzzy_matcher: Rc<SkimMatcherV2>,
        obj_choice_getter: impl Fn(&glib::Object) -> String + 'static,
    ) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();

        let ret = imp.fuzzy_matcher.set(fuzzy_matcher);
        assert!(ret.is_ok());

        let ret = imp.obj_choice_getter.set(Box::new(obj_choice_getter));
        assert!(ret.is_ok());

        this
    }

    pub fn set_search(&self, search: &str) {
        let imp = self.imp();

        if search == imp.search.borrow().as_str() {
            return;
        }

        imp.search.replace(search.to_string());
        self.changed(gtk::SorterChange::Different);
    }

    pub fn search(&self) -> String {
        self.imp().search.borrow().clone()
    }

    pub fn set_default_sorter(&self, default_sorter: Option<impl IsA<gtk::Sorter>>) {
        let imp = self.imp();

        let default_sorter = default_sorter.map(|s| s.upcast());

        if default_sorter.as_ref() == imp.default_sorter.borrow().as_ref() {
            return;
        }

        if let Some(prev_default_sorter) = imp.default_sorter.take() {
            let handler_id = imp.default_sorter_changed_id.take().unwrap();
            prev_default_sorter.disconnect(handler_id);
        }

        if let Some(default_sorter) = default_sorter {
            let changed_id = default_sorter.connect_changed(clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, changed| {
                    let imp = obj.imp();
                    if imp.search.borrow().is_empty() {
                        obj.changed(changed);
                    }
                }
            ));
            imp.default_sorter_changed_id.replace(Some(changed_id));

            imp.default_sorter.replace(Some(default_sorter));
        }

        if imp.search.borrow().is_empty() {
            self.changed(gtk::SorterChange::Different);
        }
    }

    pub fn default_sorter(&self) -> Option<gtk::Sorter> {
        self.imp().default_sorter.borrow().clone()
    }
}
