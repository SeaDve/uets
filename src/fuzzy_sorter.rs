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
        pub(super) fallback_sorter: RefCell<Option<gtk::Sorter>>,

        pub(super) fuzzy_matcher: OnceCell<Rc<SkimMatcherV2>>,
        pub(super) obj_choice_getter: OnceCell<Box<dyn Fn(&glib::Object) -> String>>,

        pub(super) fallback_sorter_changed_id: RefCell<Option<glib::SignalHandlerId>>,
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
                self.compare_fallback(obj_1, obj_2)
            } else {
                let choice_getter = self.obj_choice_getter.get().unwrap();
                let choice_1 = choice_getter(obj_1);
                let choice_2 = choice_getter(obj_2);

                let fuzzy_matcher = self.fuzzy_matcher.get().unwrap();
                let score_1 = fuzzy_matcher.fuzzy_match(&choice_1, &search);
                let score_2 = fuzzy_matcher.fuzzy_match(&choice_2, &search);

                score_2
                    .cmp(&score_1)
                    .then_with(|| self.compare_fallback(obj_1, obj_2).into())
                    .into()
            }
        }

        fn order(&self) -> gtk::SorterOrder {
            if self.search.borrow().is_empty() {
                if let Some(fallback_sorter) = self.fallback_sorter.borrow().as_ref() {
                    fallback_sorter.order()
                } else {
                    gtk::SorterOrder::None
                }
            } else {
                gtk::SorterOrder::Partial
            }
        }
    }

    impl FuzzySorter {
        fn compare_fallback(&self, obj_1: &glib::Object, obj_2: &glib::Object) -> gtk::Ordering {
            if let Some(fallback_sorter) = self.fallback_sorter.borrow().as_ref() {
                fallback_sorter.compare(obj_1, obj_2)
            } else {
                gtk::Ordering::Equal
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

    pub fn set_fallback_sorter(&self, fallback_sorter: Option<impl IsA<gtk::Sorter>>) {
        let imp = self.imp();

        let fallback_sorter = fallback_sorter.map(|s| s.upcast());

        if fallback_sorter.as_ref() == imp.fallback_sorter.borrow().as_ref() {
            return;
        }

        if let Some(prev_fallback_sorter) = imp.fallback_sorter.take() {
            let handler_id = imp.fallback_sorter_changed_id.take().unwrap();
            prev_fallback_sorter.disconnect(handler_id);
        }

        if let Some(fallback_sorter) = fallback_sorter {
            let changed_id = fallback_sorter.connect_changed(clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, changed| {
                    obj.changed(changed);
                }
            ));
            imp.fallback_sorter_changed_id.replace(Some(changed_id));

            imp.fallback_sorter.replace(Some(fallback_sorter));
        }

        self.changed(gtk::SorterChange::Different);
    }

    pub fn fallback_sorter(&self) -> Option<gtk::Sorter> {
        self.imp().fallback_sorter.borrow().clone()
    }
}
