use std::collections::HashSet;

use adw::{prelude::*, subclass::prelude::*};
use futures_channel::oneshot;
use gtk::glib::{self, closure};

use crate::{
    date_time_boxed::DateTimeBoxed,
    entity_data::{EntityData, EntityDataField, EntityDataFieldTy},
    entity_id::EntityId,
    list_model_enum,
    sex::Sex,
    stock::Stock,
    ui::{camera_viewfinder::CameraViewfinder, date_time_button::DateTimeButton},
    utils, Application,
};

list_model_enum!(Sex);

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/entity_data_dialog.ui")]
    pub struct EntityDataDialog {
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) stock_id_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) location_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) expiration_dt_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) expiration_dt_button: TemplateChild<DateTimeButton>,
        #[template_child]
        pub(super) name_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) sex_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) email_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) program_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) photo_viewfinder_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) photo_viewfinder: TemplateChild<CameraViewfinder>,

        pub(super) result_tx: RefCell<Option<oneshot::Sender<()>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityDataDialog {
        const NAME: &'static str = "UetsEntityDataDialog";
        type Type = super::EntityDataDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("entity-data-dialog.cancel", None, move |obj, _, _| {
                let imp = obj.imp();

                let _ = imp.result_tx.take().unwrap();
            });
            klass.install_action("entity-data-dialog.done", None, move |obj, _, _| {
                let imp = obj.imp();

                imp.result_tx.take().unwrap().send(()).unwrap();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EntityDataDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let stock_sorter = utils::new_sorter::<Stock>(false, |a, b| a.id().cmp(b.id()));
            let sorted_stock_model = gtk::SortListModel::new(
                Some(Application::get().timeline().stock_list().clone()),
                Some(stock_sorter),
            );

            self.stock_id_row
                .set_expression(Some(gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|stock: &Stock| stock.id().to_string()),
                )));
            self.stock_id_row.set_model(Some(&sorted_stock_model));

            self.sex_row
                .set_expression(Some(&adw::EnumListItem::this_expression("name")));
            self.sex_row.set_model(Some(&Sex::new_model()));

            self.photo_viewfinder
                .set_camera(Some(Application::get().camera().clone()));
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for EntityDataDialog {}
    impl AdwDialogImpl for EntityDataDialog {}
}

glib::wrapper! {
    pub struct EntityDataDialog(ObjectSubclass<imp::EntityDataDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl EntityDataDialog {
    pub async fn gather_data(
        entity_id: &EntityId,
        initial_data: &EntityData,
        ignored_data_field_ty: impl IntoIterator<Item = EntityDataFieldTy>,
        parent: Option<&impl IsA<gtk::Widget>>,
    ) -> Result<EntityData, oneshot::Canceled> {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.window_title.set_subtitle(&entity_id.to_string());

        let operation_mode = Application::get().settings().operation_mode();
        let ignored_data_field_tys = ignored_data_field_ty.into_iter().collect::<HashSet<_>>();
        for field_ty in EntityDataFieldTy::all() {
            let widget = match field_ty {
                EntityDataFieldTy::StockId => imp.stock_id_row.upcast_ref::<gtk::Widget>(),
                EntityDataFieldTy::Location => imp.location_row.upcast_ref(),
                EntityDataFieldTy::ExpirationDt => imp.expiration_dt_row.upcast_ref(),
                EntityDataFieldTy::Photo => imp.photo_viewfinder_group.upcast_ref(),
                EntityDataFieldTy::Name => imp.name_row.upcast_ref(),
                EntityDataFieldTy::Sex => imp.sex_row.upcast_ref(),
                EntityDataFieldTy::Email => imp.email_row.upcast_ref(),
                EntityDataFieldTy::Program => imp.program_row.upcast_ref(),
            };
            widget.set_visible(
                operation_mode.is_valid_entity_data_field_ty(*field_ty)
                    && !ignored_data_field_tys.contains(field_ty),
            );
        }

        for field in initial_data.fields() {
            match field {
                EntityDataField::StockId(stock_id) => {
                    if let Some(position) = imp
                        .stock_id_row
                        .model()
                        .unwrap()
                        .iter::<glib::Object>()
                        .position(|o| {
                            let stock = o.unwrap().downcast::<Stock>().unwrap();
                            stock.id() == stock_id
                        })
                    {
                        imp.stock_id_row.set_selected(position as u32);
                    }
                }
                EntityDataField::Location(location) => {
                    imp.location_row.set_text(location);
                }
                EntityDataField::ExpirationDt(dt) => {
                    imp.expiration_dt_button.set_dt(Some(DateTimeBoxed(*dt)));
                }
                EntityDataField::Photo(image) => {
                    imp.photo_viewfinder.set_capture_image(Some(image.clone()));
                }
                EntityDataField::Name(name) => {
                    imp.name_row.set_text(name);
                }
                EntityDataField::Sex(sex) => {
                    imp.sex_row.set_selected(sex.model_position());
                }
                EntityDataField::Email(email) => {
                    imp.email_row.set_text(email);
                }
                EntityDataField::Program(program) => {
                    imp.program_row.set_text(program);
                }
            }
        }

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
                operation_mode
                    .is_valid_entity_data_field_ty(EntityDataFieldTy::StockId)
                    .then(|| {
                        imp.stock_id_row
                            .selected_item()
                            .map(|stock| stock.downcast::<Stock>().unwrap().id().clone())
                            .map(EntityDataField::StockId)
                    })
                    .flatten(),
                Some(imp.location_row.text().to_string())
                    .filter(|t| !t.is_empty())
                    .map(EntityDataField::Location),
                operation_mode
                    .is_valid_entity_data_field_ty(EntityDataFieldTy::ExpirationDt)
                    .then(|| {
                        imp.expiration_dt_button
                            .dt()
                            .map(|dt| EntityDataField::ExpirationDt(dt.0))
                    })
                    .flatten(),
                imp.photo_viewfinder
                    .capture_image()
                    .map(EntityDataField::Photo),
                Some(imp.name_row.text().to_string())
                    .filter(|t| !t.is_empty())
                    .map(EntityDataField::Name),
                operation_mode
                    .is_valid_entity_data_field_ty(EntityDataFieldTy::Sex)
                    .then(|| {
                        imp.sex_row
                            .selected_item()
                            .map(|item| {
                                item.downcast::<adw::EnumListItem>()
                                    .unwrap()
                                    .value()
                                    .try_into()
                                    .unwrap()
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
            tracing::warn!(?operation_mode, "Invalid entity data: {:?}", data);
        }

        data
    }
}
