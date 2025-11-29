use crate::destination::{BlockDirectionFilter, Destination, SpawnDestinationMessage};
use crate::editor::{
    DespawnMessage, GenericID, HoverState, Selection, SelectionState, delete_selection_shortcut,
    finish_hover,
};
use crate::inspector::{Inspectable, InspectorPlugin};
use crate::layout::{Connections, EntityMap, MarkerMap};
use crate::marker::{Marker, MarkerColor, MarkerKey, MarkerSpawnMessage, spawn_marker};
use crate::section::LogicalSection;
use crate::selectable::{Selectable, SelectablePlugin, SelectableType};
use crate::train::{SpawnTrainMessage, Train};
use crate::new_route::TrainSpeed;
use crate::{layout_primitives::*, section::DirectedSection, track::LAYOUT_SCALE};
use bevy::color::palettes::css::{BLUE, GREEN, RED};
use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::egui::Ui;
use bevy_inspector_egui::egui::Grid;
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_prototype_lyon::prelude::*;
use serde::{Deserialize, Serialize};

pub const BLOCK_WIDTH: f32 = 20.0;

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, Serialize, Deserialize,
)]
struct LogicalID {
    direction: BlockDirection,
    facing: Facing,
}

#[derive(Debug, Clone, Message)]
struct UpdateReverseConnections {
    block_id: BlockID,
    disallow_reversing: bool,
}

#[derive(Debug, Reflect, Default, Serialize, Deserialize, Clone)]
pub struct BlockSettings {
    #[serde(default)]
    pub passthrough: bool,
    #[serde(default)]
    pub disallow_reversing: bool,
    #[serde(default)]
    pub speed: TrainSpeed,
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Block {
    pub id: BlockID,
    section: DirectedSection,
    pub settings: BlockSettings,
}

#[derive(Component)]
pub struct DirectedBlock {
    pub id: BlockID,
    pub direction: BlockDirection,
}

#[derive(Component)]
pub struct LogicalBlock {
    pub id: BlockID,
    pub direction: BlockDirection,
    pub facing: Facing,
}

impl LogicalBlock {
    pub fn to_logical_id(&self) -> LogicalBlockID {
        LogicalBlockID {
            block: self.id,
            direction: self.direction,
            facing: self.facing,
        }
    }

    pub fn in_track(&self) -> LogicalTrackID {
        self.to_logical_id().default_in_marker_track()
    }
}

#[derive(Component)]
#[relationship(relationship_target=DirectedVersions)]
struct DirectedVersionOf(Entity);

#[derive(Component)]
#[relationship_target(relationship=DirectedVersionOf)]
struct DirectedVersions(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target=LogicalVersions)]
struct LogicalVersionOf(Entity);

#[derive(Component)]
#[relationship_target(relationship=LogicalVersionOf)]
struct LogicalVersions(Vec<Entity>);

#[derive(Component)]
#[relationship_target(relationship=BlockSectionEnd)]
struct BlockSectionEndOf(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target=BlockSectionEndOf)]
struct BlockSectionEnd(Entity);

#[derive(Component)]
#[relationship(relationship_target=InTrackOf)]
pub struct InTrack(Entity);

#[derive(Component)]
#[relationship_target(relationship=InTrack)]
pub struct InTrackOf(Vec<Entity>);

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
            MessageWriter<SpawnTrainMessage>,
            Query<(&mut Destination, &Name)>,
            MessageWriter<SpawnDestinationMessage>,
            MessageWriter<UpdateReverseConnections>,
        )>::new(world);
        let (
            mut blocks,
            entity_map,
            selection_state,
            type_registry,
            mut train_spawner,
            mut destinations,
            mut destination_spawner,
            mut update_reverse_connections,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut block) = blocks.get_mut(entity) {
                ui.label(format!("Block {:?}", block.id));
                Grid::new("settings").show(ui, |ui| {
                    ui.label("Passthrough");
                    ui_for_value(&mut block.settings.passthrough, ui, &type_registry.read());
                    ui.end_row();
                    ui.label("Disallow reversing");
                    if ui_for_value(
                        &mut block.settings.disallow_reversing,
                        ui,
                        &type_registry.read(),
                    ) {
                        update_reverse_connections.write(UpdateReverseConnections {
                            block_id: block.id,
                            disallow_reversing: block.settings.disallow_reversing,
                        });
                    }
                    ui.label("Speed");
                    ui_for_value(&mut block.settings.speed, ui, &type_registry.read());
                    ui.end_row();
                });

                if ui.button("Add train").clicked() {
                    let train_id = entity_map.new_train_id();
                    let logical_block_id = block
                        .id
                        .to_logical(BlockDirection::Aligned, Facing::Forward);
                    let train = Train::at_block_id(train_id, logical_block_id);
                    train_spawner.write(SpawnTrainMessage {
                        train: train,
                        ble_train: None,
                        name: None,
                        schedule: None,
                    });
                }
                ui.separator();

                ui.heading("Destinations");
                ui.label("Assigned to destinations:");
                Grid::new("dests").show(ui, |ui| {
                    for (mut dest, dest_name) in destinations.iter_mut() {
                        if let Some(filter) = dest.get_block_filter(block.id) {
                            ui.label(dest_name.to_string());

                            let mut mutable_filter = filter.clone();
                            ui.push_id(dest.id, |ui| {
                                ui_for_value(&mut mutable_filter, ui, &type_registry.read());
                            });
                            if mutable_filter != filter {
                                dest.change_filter(block.id, mutable_filter);
                            }

                            if ui.button("X").clicked() {
                                dest.remove_block(block.id);
                            }
                            ui.end_row();
                        }
                    }
                });
                ui.label("Unassigned destinations:");
                for (mut dest, dest_name) in destinations.iter_mut() {
                    // ui.push_id(dest.id, |ui| {
                    if !dest.contains_block(block.id) {
                        if ui
                            .button(format!("Add to {}", dest_name.to_string()))
                            .clicked()
                        {
                            dest.add_block(
                                block.id,
                                crate::destination::BlockDirectionFilter::Any,
                                None,
                            );
                        }
                    }
                    //});
                }
                if ui.button("Add to new Destination").clicked() {
                    let dest_id = entity_map.new_destination_id();
                    let dest = Destination {
                        id: dest_id,
                        blocks: vec![(block.id, BlockDirectionFilter::Any, None)],
                    };
                    destination_spawner.write(SpawnDestinationMessage {
                        dest: dest,
                        name: None,
                    });
                }
            }
        }
    }
}

impl Inspectable for Block {
    fn inspector(ui: &mut Ui, world: &mut World) {
        Block::inspector(ui, world);
    }

    fn run_condition(selection_state: Res<SelectionState>) -> bool {
        selection_state.selected_type() == Some(SelectableType::Block)
    }
}

impl Selectable for Block {
    type SpawnMessage = BlockSpawnMessage;
    type ID = BlockID;

    fn get_type() -> SelectableType {
        SelectableType::Block
    }

    fn get_depth(&self) -> f32 {
        0.0
    }

    fn generic_id(&self) -> GenericID {
        GenericID::Block(self.id)
    }

    fn get_distance(
        &self,
        pos: Vec2,
        _transform: Option<&Transform>,
        _shape: Option<&Shape>,
    ) -> f32 {
        let block_dist = self.distance_to(pos) - BLOCK_WIDTH / LAYOUT_SCALE;
        block_dist
    }

    fn id(&self) -> Self::ID {
        self.id
    }
}

#[derive(Bundle)]
pub struct BlockBundle {
    shape: Shape,
    block: Block,
}

impl BlockBundle {
    pub fn new(section: DirectedSection) -> Self {
        Self::from_block(Block::new(section))
    }

    pub fn from_block(block: Block) -> Self {
        let shape = generate_block_shape(&block.section);

        let shape = ShapeBuilder::with(&shape)
            .stroke(Stroke {
                color: Color::from(GREEN),
                options: StrokeOptions::default()
                    .with_line_width(BLOCK_WIDTH)
                    .with_line_cap(LineCap::Round),
            })
            .build();

        Self {
            shape: shape,
            block: block,
        }
    }
}

fn generate_block_shape(section: &DirectedSection) -> ShapePath {
    let mut path = ShapePath::new();
    path = path.move_to(section.interpolate_pos(0.0) * LAYOUT_SCALE);

    let num_segments = 10 * section.len();
    let length = section.length();

    for i in 1..(num_segments + 1) {
        let dist = i as f32 * length / num_segments as f32;
        path = path.line_to(section.interpolate_pos(dist) * LAYOUT_SCALE);
    }
    path
}

fn update_reverse_connections(
    mut update_reverse_connections: MessageReader<UpdateReverseConnections>,
    mut connections: ResMut<Connections>,
) {
    for UpdateReverseConnections {
        block_id,
        disallow_reversing,
    } in update_reverse_connections.read()
    {
        for logical_id in block_id.logical_block_ids() {
            let in_track = logical_id.default_in_marker_track();
            if !disallow_reversing {
                println!("Connecting tracks {:?} {:?}", in_track, in_track.reversed());
                connections.connect_tracks(&in_track, &in_track.reversed());
            } else {
                println!(
                    "Disconnecting tracks {:?} {:?}",
                    in_track,
                    in_track.reversed()
                );
                connections.disconnect_tracks(&in_track, &in_track.reversed());
            }
        }
    }
}

#[derive(Debug, Message, Clone, Serialize, Deserialize)]
pub struct BlockSpawnMessage {
    pub block: Block,
    pub name: Option<String>,
}

#[derive(SystemParam)]
pub struct BlockSpawnMessageQuery<'w, 's> {
    query: Query<'w, 's, (&'static Block, &'static Name)>,
}
impl BlockSpawnMessageQuery<'_, '_> {
    pub fn get(&self) -> Vec<BlockSpawnMessage> {
        self.query
            .iter()
            .map(|(block, name)| BlockSpawnMessage {
                block: block.clone(),
                name: Some(name.to_string()),
            })
            .collect()
    }
}

#[derive(Debug, Message, Clone, Serialize, Deserialize)]
pub struct BlockCreateMessage(pub Block);

fn create_block(
    mut create_messages: MessageReader<BlockCreateMessage>,
    mut block_message_writer: MessageWriter<BlockSpawnMessage>,
    mut marker_message_writer: MessageWriter<MarkerSpawnMessage>,
    mut marker_map: ResMut<MarkerMap>,
) {
    for BlockCreateMessage(block) in create_messages.read() {
        let block_id = block.id;
        block_message_writer.write(BlockSpawnMessage {
            block: block.clone(),
            name: None,
        });
        for logical_id in block_id.logical_block_ids() {
            let in_track = logical_id.default_in_marker_track();
            if logical_id.facing == Facing::Forward {
                let marker = Marker::new(in_track.track(), MarkerColor::Any);
                marker_message_writer.write(MarkerSpawnMessage(marker));
            }
            marker_map.register_marker(in_track, MarkerKey::In, logical_id);
        }
    }
}

pub fn spawn_block(
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    mut block_event_reader: MessageReader<BlockSpawnMessage>,
    mut connections: ResMut<Connections>,
) {
    for request in block_event_reader.read() {
        println!("Spawning block {:?}", request.block.id);
        let block = request.block.clone();
        let block_id = block.id;
        // println!("Spawning block {:?}", block_id);
        let name = Name::new(request.name.clone().unwrap_or(block_id.to_string()));
        let entity = commands
            .spawn((BlockBundle::from_block(block.clone()), name))
            .id();
        entity_map.add_block(block_id, entity);
        for direction in [BlockDirection::Aligned, BlockDirection::Opposite] {
            let directed_id = DirectedBlockID {
                id: block_id,
                direction,
            };
            let end_track = directed_id.section_end_track();
            let track_entity = entity_map.tracks.get(&end_track.track).unwrap();
            let directed_entity = commands
                .spawn((
                    DirectedBlock {
                        id: block_id,
                        direction,
                    },
                    DirectedVersionOf(entity),
                    BlockSectionEnd(*track_entity),
                ))
                .id();

            for facing in [Facing::Forward, Facing::Backward] {
                let logical_id = block_id.to_logical(direction, facing);
                let in_track = logical_id.default_in_marker_track();
                let in_track_entity = entity_map.tracks.get(&in_track.track()).unwrap();
                commands.spawn((
                    LogicalBlock {
                        id: block_id,
                        direction,
                        facing,
                    },
                    LogicalVersionOf(directed_entity),
                    InTrack(*in_track_entity),
                    block.get_logical_section(logical_id),
                ));
                if !block.settings.disallow_reversing {
                    connections.connect_tracks(&in_track, &in_track.reversed());
                }
            }
        }
    }
}

pub fn despawn_block(
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    mut block_event_reader: MessageReader<DespawnMessage<Block>>,
    mut marker_map: ResMut<MarkerMap>,
    mut connections: ResMut<Connections>,
) {
    for request in block_event_reader.read() {
        let block_id = request.0;
        println!("Despawning block {:?}", block_id);
        for logical_id in block_id.logical_block_ids() {
            let in_track = logical_id.default_in_marker_track();
            connections.disconnect_tracks(&in_track, &in_track.reversed());
        }
        let entity = entity_map.blocks.get(&block_id).unwrap().clone();
        commands.entity(entity).despawn();
        entity_map.remove_block(block_id);
        marker_map.remove_block(block_id);
    }
}

fn update_block_color(
    mut q_strokes: Query<(&Block, &mut Shape)>,
    selection_state: Res<SelectionState>,
    hover_state: Res<HoverState>,
) {
    if !selection_state.is_changed() && !hover_state.is_changed() {
        return;
    }
    for (block, mut shape) in q_strokes.iter_mut() {
        if let Some(GenericID::Block(block_id)) = &hover_state.hover {
            if block.id == *block_id {
                shape.stroke.as_mut().unwrap().color = Color::from(RED);
                continue;
            }
        }
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            if block.id == *block_id {
                shape.stroke.as_mut().unwrap().color = Color::from(BLUE);
                continue;
            }
        }
        shape.stroke.as_mut().unwrap().color = Color::from(GREEN);
    }
}

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SelectablePlugin::<Block>::new());
        app.add_plugins(InspectorPlugin::<Block>::new());
        app.register_type::<Block>();
        app.add_message::<BlockSpawnMessage>();
        app.add_message::<DespawnMessage<Block>>();
        app.add_message::<BlockCreateMessage>();
        app.add_message::<UpdateReverseConnections>();
        app.add_systems(
            Update,
            (
                create_block.run_if(on_message::<BlockCreateMessage>),
                update_reverse_connections.run_if(on_message::<UpdateReverseConnections>),
                update_block_color.after(finish_hover),
                delete_selection_shortcut::<Block>,
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                spawn_block
                    .run_if(on_message::<BlockSpawnMessage>)
                    .after(spawn_marker),
                despawn_block,
            ),
        );
    }
}
