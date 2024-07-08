use crate::destination::{BlockDirectionFilter, Destination, SpawnDestinationEvent};
use crate::editor::{
    delete_selection_shortcut, DespawnEvent, GenericID, HoverState, Selectable, Selection,
    SelectionState, SpawnTrainEvent,
};
use crate::layout::{Connections, EntityMap, MarkerMap};
use crate::marker::{spawn_marker, Marker, MarkerColor, MarkerKey, MarkerSpawnEvent};
use crate::section::LogicalSection;
use crate::train::Train;
use crate::{layout_primitives::*, section::DirectedSection, track::LAYOUT_SCALE};
use bevy::color::palettes::css::{BLUE, GREEN, RED};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::egui::Ui;
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_prototype_lyon::{
    draw::Stroke,
    entity::ShapeBundle,
    path::PathBuilder,
    prelude::{LineCap, StrokeOptions},
};
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

pub const BLOCK_WIDTH: f32 = 20.0;

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, Serialize, Deserialize,
)]
struct LogicalID {
    direction: BlockDirection,
    facing: Facing,
}

#[derive(Debug, Reflect, Default, Serialize, Deserialize, Clone)]
pub struct BlockSettings {}

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Block {
    pub id: BlockID,
    section: DirectedSection,
    pub settings: BlockSettings,
}

impl Block {
    pub fn new(section: DirectedSection) -> Self {
        let id = section.to_block_id();
        let section = if &id.track1 == section.tracks.first().unwrap() {
            section
        } else {
            section.get_opposite()
        };
        let block = Block {
            id: section.to_block_id(),
            section: section,
            settings: BlockSettings::default(),
        };
        block
    }

    pub fn distance_to(&self, pos: Vec2) -> f32 {
        self.section.distance_to(pos)
    }

    pub fn hover_pos_to_direction(&self, pos: Vec2) -> BlockDirection {
        let track_index = self.section.closest_track_index(pos);
        if track_index >= self.section.len() / 2 {
            BlockDirection::Aligned
        } else {
            BlockDirection::Opposite
        }
    }

    pub fn get_logical_section(&self, block_id: LogicalBlockID) -> LogicalSection {
        match block_id.direction {
            BlockDirection::Aligned => self.section.get_logical(block_id.facing),
            BlockDirection::Opposite => self.section.get_opposite().get_logical(block_id.facing),
        }
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut Block>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
            EventWriter<SpawnTrainEvent>,
            Query<&mut Destination>,
            EventWriter<SpawnDestinationEvent>,
        )>::new(world);
        let (
            mut blocks,
            entity_map,
            selection_state,
            type_registry,
            mut train_spawner,
            mut destinations,
            mut destination_spawner,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut block) = blocks.get_mut(entity) {
                ui.label(format!("Block {:?}", block.id));
                ui_for_value(&mut block.settings, ui, &type_registry.read());

                if ui.button("Add train").clicked() {
                    let train_id = entity_map.new_train_id();
                    let logical_block_id = block
                        .id
                        .to_logical(BlockDirection::Aligned, Facing::Forward);
                    let train = Train::at_block_id(train_id, logical_block_id);
                    train_spawner.send(SpawnTrainEvent {
                        train: train,
                        ble_train: None,
                    });
                }
                ui.separator();

                ui.label("Destinations");
                for mut dest in destinations.iter_mut() {
                    ui.push_id(dest.id, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(filter) = dest.get_block_filter(block.id) {
                                ui.label(format!("Destination{}", dest.id.id));

                                let mut mutable_filter = filter.clone();
                                ui_for_value(&mut mutable_filter, ui, &type_registry.read());
                                if mutable_filter != filter {
                                    dest.change_filter(block.id, mutable_filter);
                                }

                                if ui.button("X").clicked() {
                                    dest.remove_block(block.id);
                                }
                            } else {
                                if ui
                                    .button(format!("Add to Destination{}", dest.id.id))
                                    .clicked()
                                {
                                    dest.add_block(
                                        block.id,
                                        crate::destination::BlockDirectionFilter::Any,
                                        None,
                                    );
                                }
                            }
                        });
                    });
                }
                if ui.button("Add to new Destination").clicked() {
                    let dest_id = entity_map.new_destination_id();
                    let dest = Destination {
                        id: dest_id,
                        blocks: vec![(block.id, BlockDirectionFilter::Any, None)],
                    };
                    destination_spawner.send(SpawnDestinationEvent(dest));
                }
            }
        }
    }
}

impl Selectable for Block {
    fn get_depth(&self) -> f32 {
        0.0
    }

    fn get_id(&self) -> GenericID {
        GenericID::Block(self.id)
    }

    fn get_distance(
        &self,
        pos: Vec2,
        _transform: Option<&Transform>,
        _stroke: Option<&Stroke>,
    ) -> f32 {
        let block_dist = self.distance_to(pos) - BLOCK_WIDTH / LAYOUT_SCALE;
        block_dist
    }
}

#[derive(Bundle)]
pub struct BlockBundle {
    shape: ShapeBundle,
    stroke: Stroke,
    block: Block,
}

impl BlockBundle {
    pub fn new(section: DirectedSection) -> Self {
        Self::from_block(Block::new(section))
    }

    pub fn from_block(block: Block) -> Self {
        let shape = generate_block_shape(&block.section);
        let stroke = {
            let stroke = Stroke {
                color: Color::from(GREEN),
                options: StrokeOptions::default()
                    .with_line_width(BLOCK_WIDTH)
                    .with_line_cap(LineCap::Round),
            };
            stroke
        };

        Self {
            shape: shape,
            stroke: stroke,
            block: block,
        }
    }
}

fn generate_block_shape(section: &DirectedSection) -> ShapeBundle {
    let mut path_builder = PathBuilder::new();
    path_builder.move_to(section.interpolate_pos(0.0) * LAYOUT_SCALE);

    let num_segments = 10 * section.len();
    let length = section.length();

    for i in 1..(num_segments + 1) {
        let dist = i as f32 * length / num_segments as f32;
        path_builder.line_to(section.interpolate_pos(dist) * LAYOUT_SCALE);
    }

    let path = path_builder.build();

    let shape = ShapeBundle {
        path: path,
        ..Default::default()
    };
    shape
}

#[derive(Debug, Event, Clone, Serialize, Deserialize)]
pub struct BlockSpawnEvent(pub Block);

#[derive(Debug, Event, Clone, Serialize, Deserialize)]
pub struct BlockCreateEvent(pub Block);

fn create_block(
    mut create_events: EventReader<BlockCreateEvent>,
    mut block_event_writer: EventWriter<BlockSpawnEvent>,
    mut marker_event_writer: EventWriter<MarkerSpawnEvent>,
    mut marker_map: ResMut<MarkerMap>,
) {
    for BlockCreateEvent(block) in create_events.read() {
        let block_id = block.id;
        block_event_writer.send(BlockSpawnEvent(block.clone()));
        for logical_id in block_id.logical_block_ids() {
            let in_track = logical_id.default_in_marker_track();
            if logical_id.facing == Facing::Forward {
                let marker = Marker::new(in_track.track(), MarkerColor::Any);
                marker_event_writer.send(MarkerSpawnEvent(marker));
            }
            marker_map.register_marker(in_track, MarkerKey::In, logical_id);
        }
    }
}

pub fn spawn_block(
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    mut block_event_reader: EventReader<BlockSpawnEvent>,
    mut connections: ResMut<Connections>,
) {
    for request in block_event_reader.read() {
        println!("Spawning block {:?}", request.0.id);
        let block = request.0.clone();
        let block = BlockBundle::from_block(block);
        let block_id = block.block.id;
        // println!("Spawning block {:?}", block_id);
        let name = Name::new(block_id.to_string());
        let entity = commands.spawn((block, name)).id();
        entity_map.add_block(block_id, entity);
        for logical_id in block_id.logical_block_ids() {
            let in_track = logical_id.default_in_marker_track();
            connections.connect_tracks(&in_track, &in_track.reversed());
        }
    }
}

pub fn despawn_block(
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    mut block_event_reader: EventReader<DespawnEvent<Block>>,
    mut marker_map: ResMut<MarkerMap>,
    mut connections: ResMut<Connections>,
) {
    for request in block_event_reader.read() {
        let block = request.0.clone();
        let block_id = block.id;
        println!("Despawning block {:?}", block_id);
        for logical_id in block_id.logical_block_ids() {
            let in_track = logical_id.default_in_marker_track();
            connections.disconnect_tracks(&in_track, &in_track.reversed());
        }
        let entity = entity_map.blocks.get(&block_id).unwrap().clone();
        commands.entity(entity).despawn_recursive();
        entity_map.remove_block(block_id);
        marker_map.remove_block(block_id);
    }
}

fn update_block_color(
    mut q_strokes: Query<(&Block, &mut Stroke)>,
    selection_state: Res<SelectionState>,
    hover_state: Res<HoverState>,
) {
    if !selection_state.is_changed() && !hover_state.is_changed() {
        return;
    }
    for (block, mut stroke) in q_strokes.iter_mut() {
        if let Some(GenericID::Block(block_id)) = &hover_state.hover {
            if block.id == *block_id {
                stroke.color = Color::from(RED);
                continue;
            }
        }
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            if block.id == *block_id {
                stroke.color = Color::from(BLUE);
                continue;
            }
        }
        stroke.color = Color::from(GREEN);
    }
}

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Block>();
        app.register_component_as::<dyn Selectable, Block>();
        app.add_event::<BlockSpawnEvent>();
        app.add_event::<DespawnEvent<Block>>();
        app.add_event::<BlockCreateEvent>();
        app.add_systems(
            Update,
            (
                create_block.run_if(on_event::<BlockCreateEvent>()),
                update_block_color,
                delete_selection_shortcut::<Block>,
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                spawn_block
                    .run_if(on_event::<BlockSpawnEvent>())
                    .after(spawn_marker),
                despawn_block,
            ),
        );
    }
}
