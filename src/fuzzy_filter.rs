use std::cell::RefCell;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gtk::{glib, prelude::*, subclass::prelude::*};

use std::rc::Rc;

use crate::fuzzy_sorter::FuzzySorter;

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct FuzzyFilter {
        pub(super) search: RefCell<String>,

        pub(super) obj_choice_getter: OnceCell<Box<dyn Fn(&glib::Object) -> String>>,
        pub(super) sorter: OnceCell<FuzzySorter>,

        pub(super) fuzzy_matcher: Rc<SkimMatcherV2>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FuzzyFilter {
        const NAME: &'static str = "UetsFuzzyFilter";
        type Type = super::FuzzyFilter;
        type ParentType = gtk::Filter;
    }

    impl ObjectImpl for FuzzyFilter {}

    impl FilterImpl for FuzzyFilter {
        fn strictness(&self) -> gtk::FilterMatch {
            if self.search.borrow().is_empty() {
                gtk::FilterMatch::All
            } else {
                gtk::FilterMatch::Some
            }
        }

        fn match_(&self, obj: &glib::Object) -> bool {
            let search = self.search.borrow();

            if search.is_empty() {
                true
            } else {
                let choice_getter = self.obj_choice_getter.get().unwrap();
                let choice = choice_getter(obj);

                self.fuzzy_matcher.fuzzy_match(&choice, &search).is_some()
            }
        }
    }
}

glib::wrapper! {
    pub struct FuzzyFilter(ObjectSubclass<imp::FuzzyFilter>)
        @extends gtk::Filter;

}

impl FuzzyFilter {
    pub fn new(obj_choice_getter: impl Fn(&glib::Object) -> String + Clone + 'static) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();

        let ret = imp
            .obj_choice_getter
            .set(Box::new(obj_choice_getter.clone()));
        assert!(ret.is_ok());

        imp.sorter
            .set(FuzzySorter::new(
                imp.fuzzy_matcher.clone(),
                obj_choice_getter,
            ))
            .unwrap();

        this
    }

    pub fn sorter(&self) -> &FuzzySorter {
        self.imp().sorter.get().unwrap()
    }

    pub fn set_search(&self, search: &str) {
        let imp = self.imp();

        let old_search = self.search();
        let search = search.to_lowercase();

        if old_search == search {
            return;
        }

        let change = if search.is_empty() {
            gtk::FilterChange::LessStrict
        } else if search.starts_with(&old_search) {
            gtk::FilterChange::MoreStrict
        } else if old_search.starts_with(&search) {
            gtk::FilterChange::LessStrict
        } else {
            gtk::FilterChange::Different
        };

        imp.search.replace(search.clone());
        self.changed(change);

        self.sorter().set_search(&search);
    }

    pub fn search(&self) -> String {
        self.imp().search.borrow().clone()
    }
}
