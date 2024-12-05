use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone};

use crate::{detected_wo_id_list::DetectedWoIdList, ui::detected_wo_id_row::DetectedWoIdRow};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/detected_wo_id_dialog.ui")]
    pub struct DetectedWoIdDialog {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) empty_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) main_page: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection_model: TemplateChild<gtk::SelectionModel>,
        #[template_child]
        pub(super) sort_list_model: TemplateChild<gtk::SortListModel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetectedWoIdDialog {
        const NAME: &'static str = "UetsDetectedWoIdDialog";
        type Type = super::DetectedWoIdDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DetectedWoIdDialog {
        fn constructed(&self) {
            self.parent_constructed();

            self.list_view.remove_css_class("view");

            let obj = self.obj();

            self.selection_model.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
                }
            ));

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(|_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_selectable(false);
                list_item.set_activatable(false);

                let row = DetectedWoIdRow::new();

                list_item
                    .property_expression("item")
                    .bind(&row, "item", glib::Object::NONE);

                list_item.set_child(Some(&row));
            });
            self.list_view.set_factory(Some(&factory));

            obj.update_stack();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for DetectedWoIdDialog {}
    impl AdwDialogImpl for DetectedWoIdDialog {}
}

glib::wrapper! {
    pub struct DetectedWoIdDialog(ObjectSubclass<imp::DetectedWoIdDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl DetectedWoIdDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_model(&self, list: Option<&DetectedWoIdList>) {
        let imp = self.imp();
        imp.sort_list_model.set_model(list);
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.selection_model.n_items() == 0 {
            imp.stack.set_visible_child(&*imp.empty_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }
}
