use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_inspector_egui::egui::{self, ComboBox};
use bevy_prototype_lyon::draw::Stroke;

use crate::{
    editor::{finish_hover, init_hover, update_hover, GenericID},
    layout::EntityMap,
};

pub struct SelectablePlugin<T: Selectable> {
    _phantom: PhantomData<T>,
}

impl<T: Selectable> SelectablePlugin<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: Selectable> Plugin for SelectablePlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_hover::<T>.after(init_hover).before(finish_hover),
        );
    }
}

pub trait Selectable: Sync + Send + 'static + Component {
    type SpawnEvent: Event;
    type ID: PartialEq + Eq + Clone + Copy + std::fmt::Debug + Send + Sync;

    fn generic_id(&self) -> GenericID;

    fn id(&self) -> Self::ID;

    fn get_depth(&self) -> f32 {
        -100.0
    }

    fn get_distance(
        &self,
        _pos: Vec2,
        _transform: Option<&Transform>,
        _stroke: Option<&Stroke>,
    ) -> f32 {
        100.0
    }

    fn name(&self) -> String {
        format!("{:}", self.generic_id())
    }

    fn default_spawn_event(_entity_map: &mut ResMut<EntityMap>) -> Option<Self::SpawnEvent> {
        None
    }

    fn selector_option(
        query: &Query<(&Self, Option<&Name>)>,
        ui: &mut egui::Ui,
        value: &mut Option<Self::ID>,
    ) where
        Self: Component + Sized,
    {
        let selected_text = Self::label_from_query(value, query);
        ComboBox::from_id_salt("selector")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(value, None, "None".to_string());
                for (selectable, name) in query.iter() {
                    ui.selectable_value(
                        value,
                        Some(selectable.id()),
                        name.map_or(selectable.generic_id().to_string(), |v| v.to_string()),
                    );
                }
            });
    }

    fn selector(query: &Query<(&Self, Option<&Name>)>, ui: &mut egui::Ui, value: &mut Self::ID)
    where
        Self: Component + Sized,
    {
        let selected_text = Self::label_from_query(&Some(value.clone()), query);
        ComboBox::from_id_salt("selector")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for (selectable, name) in query.iter() {
                    ui.selectable_value(
                        value,
                        selectable.id(),
                        name.map_or(selectable.generic_id().to_string(), |v| v.to_string()),
                    );
                }
            });
    }

    fn label_from_query(
        value: &Option<<Self as Selectable>::ID>,
        query: &Query<(&Self, Option<&Name>)>,
    ) -> String
    where
        Self: Component + Sized,
    {
        let selected_text = value.map_or("None".to_string(), |v| {
            query
                .iter()
                .find_map(|(selectable, name)| {
                    if selectable.id() == v {
                        Some(name.map_or(selectable.generic_id().to_string(), |v| v.to_string()))
                    } else {
                        None
                    }
                })
                .unwrap_or("Not found!!".to_string())
        });
        selected_text
    }
}
