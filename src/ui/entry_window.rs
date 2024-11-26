use adw::{prelude::*, subclass::prelude::*};
use futures_channel::oneshot;
use gtk::{
    gio,
    glib::{self, closure, BoxedAnyObject},
};

use crate::{entity_data::EntityData, stock_id::StockId, Application};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/entry_window.ui")]
    pub struct EntryWindow {
        #[template_child]
        pub(super) stock_id_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) stock_id_dropdown: TemplateChild<gtk::DropDown>,

        pub(super) result_tx: RefCell<Option<oneshot::Sender<()>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntryWindow {
        const NAME: &'static str = "UetsEntryWindow";
        type Type = super::EntryWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("entry-window.cancel", None, move |obj, _, _| {
                let imp = obj.imp();

                let _ = imp.result_tx.take().unwrap();
            });
            klass.install_action("entry-window.done", None, move |obj, _, _| {
                let imp = obj.imp();

                imp.result_tx.take().unwrap().send(()).unwrap();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EntryWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let operation_mode = Application::get().settings().operation_mode();

            self.stock_id_row.set_visible(operation_mode.has_stocks());

            let app = Application::get();
            let stock_ids = {
                let mut vec = app
                    .timeline()
                    .stock_list()
                    .iter()
                    .map(|stock| stock.id().clone())
                    .chain(app.entity_data_index().retrieve_stock_ids())
                    .collect::<Vec<_>>();
                vec.sort_unstable();
                vec.dedup();
                vec
            };

            let stock_id_model = gio::ListStore::new::<BoxedAnyObject>();
            stock_id_model.splice(
                0,
                0,
                &stock_ids
                    .into_iter()
                    .map(BoxedAnyObject::new)
                    .collect::<Vec<_>>(),
            );

            self.stock_id_dropdown
                .set_expression(Some(gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|o: &BoxedAnyObject| o.borrow::<StockId>().to_string()),
                )));
            self.stock_id_dropdown.set_model(Some(&stock_id_model));
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for EntryWindow {}
    impl WindowImpl for EntryWindow {}
    impl AdwWindowImpl for EntryWindow {}
}

glib::wrapper! {
    pub struct EntryWindow(ObjectSubclass<imp::EntryWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl EntryWindow {
    pub async fn gather_data(
        parent: &impl IsA<gtk::Widget>,
    ) -> Result<EntityData, oneshot::Canceled> {
        let root = parent.root().map(|r| r.downcast::<gtk::Window>().unwrap());

        let this = glib::Object::builder::<Self>()
            .property("transient-for", root)
            .property("modal", true)
            .build();

        let imp = this.imp();

        let (result_tx, result_rx) = oneshot::channel();
        imp.result_tx.replace(Some(result_tx));

        this.present();

        if let Err(err @ oneshot::Canceled) = result_rx.await {
            this.close();
            return Err(err);
        }

        this.close();

        Ok(this.gather_data_inner())
    }

    fn gather_data_inner(&self) -> EntityData {
        let imp = self.imp();

        let stock_id = imp.stock_id_dropdown.selected_item().map(|stock| {
            stock
                .downcast::<BoxedAnyObject>()
                .unwrap()
                .borrow::<StockId>()
                .clone()
        });

        EntityData { stock_id }
    }
}
