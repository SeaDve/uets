use adw::{prelude::*, subclass::prelude::*};
use futures_channel::oneshot;
use gtk::glib::{self, closure};

use crate::{
    entity_data::{EntityData, EntityDataField, EntityDataFieldTy},
    stock::Stock,
    utils, Application,
};

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
        #[template_child]
        pub(super) location_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) expiration_dt_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) name_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) sex_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) email_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) program_row: TemplateChild<adw::EntryRow>,

        pub(super) result_tx: RefCell<Option<oneshot::Sender<()>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntryWindow {
        const NAME: &'static str = "UetsEntryWindow";
        type Type = super::EntryWindow;
        type ParentType = adw::Dialog;

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

            let mode = Application::get().settings().operation_mode();
            self.stock_id_row
                .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::StockId));
            self.location_row
                .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::Location));
            self.expiration_dt_row
                .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::ExpirationDt));
            self.name_row
                .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::Name));
            self.sex_row
                .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::Sex));
            self.email_row
                .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::Email));
            self.program_row
                .set_visible(mode.is_valid_entity_data_field_ty(EntityDataFieldTy::Program));

            let stock_sorter = utils::new_sorter::<Stock>(false, |a, b| a.id().cmp(b.id()));
            let sorted_stock_model = gtk::SortListModel::new(
                Some(Application::get().timeline().stock_list().clone()),
                Some(stock_sorter),
            );

            self.stock_id_dropdown
                .set_expression(Some(gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|stock: &Stock| stock.id().to_string()),
                )));
            self.stock_id_dropdown.set_model(Some(&sorted_stock_model));
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for EntryWindow {}
    impl AdwDialogImpl for EntryWindow {}
}

glib::wrapper! {
    pub struct EntryWindow(ObjectSubclass<imp::EntryWindow>)
        @extends gtk::Widget, adw::Dialog;
}

impl EntryWindow {
    pub async fn gather_data(
        parent: Option<&impl IsA<gtk::Widget>>,
    ) -> Result<EntityData, oneshot::Canceled> {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();

        let (result_tx, result_rx) = oneshot::channel();
        imp.result_tx.replace(Some(result_tx));

        this.present(parent);

        if let Err(err @ oneshot::Canceled) = result_rx.await {
            this.close();
            return Err(err);
        }

        this.close();

        Ok(this.gather_data_inner())
    }

    fn gather_data_inner(&self) -> EntityData {
        let imp = self.imp();

        let operation_mode = Application::get().settings().operation_mode();

        let data = EntityData::from_fields(
            [
                imp.stock_id_dropdown
                    .selected_item()
                    .map(|stock| stock.downcast::<Stock>().unwrap().id().clone())
                    .map(EntityDataField::StockId),
                Some(imp.location_row.text().to_string())
                    .filter(|t| !t.is_empty())
                    .map(EntityDataField::Location),
                Some(imp.expiration_dt_row.text().to_string())
                    .filter(|t| !t.is_empty())
                    .map(EntityDataField::ExpirationDt),
                Some(imp.name_row.text().to_string())
                    .filter(|t| !t.is_empty())
                    .map(EntityDataField::Name),
                operation_mode
                    .is_valid_entity_data_field_ty(EntityDataFieldTy::Sex)
                    .then(|| {
                        imp.sex_row
                            .selected_item()
                            .map(|item| {
                                item.downcast::<gtk::StringObject>()
                                    .unwrap()
                                    .string()
                                    .to_string()
                            })
                            .map(EntityDataField::Sex)
                    })
                    .flatten(),
                Some(imp.email_row.text().to_string())
                    .filter(|t| !t.is_empty())
                    .map(EntityDataField::Email),
                Some(imp.program_row.text().to_string())
                    .filter(|t| !t.is_empty())
                    .map(EntityDataField::Program),
            ]
            .into_iter()
            .flatten(),
        );

        if !operation_mode.is_valid_entity_data(&data) {
            tracing::debug!(?operation_mode, "Invalid entity data: {:?}", data);
        }

        data
    }
}
