//! Reversible BSN Editor authoring and viewport integration for Aqua.
//!
//! External builds use BSN's contracted SDK-source redirect so this facade's
//! Git `bsn_extension` edge resolves to the exact ABI-v7 host artifact.

use std::any::TypeId;

use bevy::{core_pipeline::prepass::DepthPrepass, prelude::*};
use bevy_aqua::AquaPrimaryView;
use bevy_aqua_bsn::{
    AquaBsnPlugin, AquaCausticsSettings, AquaOceanSettings, AquaOpticsPreset,
    AquaReflectionSettings, AquaRiverPoint, AquaSeaState, AquaWaterBodySettings, AquaWaterShape,
    AquaWaveModel,
};
use bsn_extension::{inspector::InspectorCategory, prelude::*, selection::SelectionRequest};

pub const AQUA_EXTENSION_ID: &str = "bevy_aqua.editor";

// BSN activates and tears down linked generations inside Update. The active
// schedule is temporarily absent from World, so removable extension systems
// must live in a different schedule. PreUpdate also adapts the camera before
// Aqua selects its primary view in PostUpdate.
const VIEWPORT_SYNC_SCHEDULE: PreUpdate = PreUpdate;

#[derive(Default, Debug, Clone, Copy)]
pub struct AquaEditorExtension;

impl BsnExtension for AquaEditorExtension {
    fn id(&self) -> String {
        AQUA_EXTENSION_ID.into()
    }

    fn label(&self) -> String {
        "Aqua Water".into()
    }

    fn description(&self) -> String {
        "Ocean, lake and river authoring with live Aqua viewport preview.".into()
    }

    fn bootstrap_requirement(&self) -> ExtensionBootstrapRequirement {
        ExtensionBootstrapRequirement::StartupRequired
    }

    fn bootstrap(&self, ctx: &mut ExtensionBootstrapContext) -> ExtensionResult {
        initialize_extension_task_pools();
        ctx.install_startup_plugin(AquaBsnPlugin)
            .register_type::<AquaWaveModel>()
            .register_type::<AquaSeaState>()
            .register_type::<AquaOpticsPreset>()
            .register_type::<AquaReflectionSettings>()
            .register_type::<AquaCausticsSettings>()
            .register_type::<AquaOceanSettings>()
            .register_type::<AquaRiverPoint>()
            .register_type::<AquaWaterShape>()
            .register_type::<AquaWaterBodySettings>();
        Ok(())
    }

    fn register(&self, ctx: &mut ExtensionContext) -> ExtensionResult {
        ctx.add_systems(
            VIEWPORT_SYNC_SCHEDULE,
            (sync_aqua_viewport_camera, ApplyDeferred).chain(),
        )?;
        ctx.register_operator::<AquaAddOceanOp>()
            .register_operator::<AquaAddLakeOp>()
            .register_operator::<AquaAddRiverOp>()
            .register_menu_entry::<AquaAddOceanOp>(TopLevelMenu::Add)
            .register_menu_entry::<AquaAddLakeOp>(TopLevelMenu::Add)
            .register_menu_entry::<AquaAddRiverOp>(TopLevelMenu::Add);
        ctx.register_inspector_category(InspectorCategory {
            id: "aqua".into(),
            label: "Aqua".into(),
            icon: Icon::Waves,
            order: 46,
        })?;
        ctx.register_component_category::<AquaOceanSettings>("aqua")?;
        ctx.register_component_category::<AquaWaterBodySettings>("aqua")?;
        ctx.register_entity_icon("bevy_aqua_bsn::AquaOceanSettings", Icon::Waves)
            .register_entity_icon("bevy_aqua_bsn::AquaWaterBodySettings", Icon::Waves);
        Ok(())
    }

    fn unregister(&self, ctx: &mut ExtensionUnregisterContext) -> ExtensionResult {
        ctx.run_once(restore_aqua_viewport_cameras)?;
        Ok(())
    }
}

/// Initialize the Bevy task-pool statics linked into this native generation.
///
/// BSN redirects the extension to the host's exact Bevy rlibs for Rust type
/// identity, but an EXE and a DLL still receive separate copies of crate-level
/// statics. Aqua loads embedded shaders while its startup plugin is built, so
/// the DLL-local `IoTaskPool` must exist before `AquaBsnPlugin` is installed.
/// `create_default_pools` is idempotent when a future linking mode shares those
/// statics with the host.
fn initialize_extension_task_pools() {
    TaskPoolOptions::default().create_default_pools();
}

#[derive(Component, Clone, Copy, Debug)]
struct AquaEditorCamera {
    added_primary: bool,
    added_depth: bool,
}

fn sync_aqua_viewport_camera(
    mode: Option<Res<EditorViewMode>>,
    viewport: Option<Res<EditorViewportState>>,
    oceans: Query<(), With<AquaOceanSettings>>,
    bodies: Query<(), With<AquaWaterBodySettings>>,
    cameras: Query<
        (
            Entity,
            Has<AquaPrimaryView>,
            Has<DepthPrepass>,
            Option<&AquaEditorCamera>,
        ),
        With<MainViewportCamera>,
    >,
    mut commands: Commands,
) {
    let scene_mode = mode
        .as_deref()
        .is_none_or(|mode| *mode == EditorViewMode::Scene);
    let has_authored_water = !oceans.is_empty() || !bodies.is_empty();
    let target = (scene_mode && has_authored_water)
        .then(|| viewport.as_deref().and_then(|state| state.camera_entity))
        .flatten();

    for (entity, has_primary, has_depth, owned) in &cameras {
        if Some(entity) == target {
            if let Some(owned) = owned {
                let mut camera = commands.entity(entity);
                if owned.added_primary && !has_primary {
                    camera.insert(AquaPrimaryView);
                }
                if owned.added_depth && !has_depth {
                    camera.insert(DepthPrepass);
                }
            } else {
                let ownership = AquaEditorCamera {
                    added_primary: !has_primary,
                    added_depth: !has_depth,
                };
                let mut camera = commands.entity(entity);
                if ownership.added_primary {
                    camera.insert(AquaPrimaryView);
                }
                if ownership.added_depth {
                    camera.insert(DepthPrepass);
                }
                camera.insert(ownership);
            }
            continue;
        }

        if let Some(owned) = owned {
            remove_owned_camera_adaptation(&mut commands.entity(entity), *owned);
        }
    }
}

fn remove_owned_camera_adaptation(camera: &mut EntityCommands, owned: AquaEditorCamera) {
    if owned.added_primary {
        camera.remove::<AquaPrimaryView>();
    }
    if owned.added_depth {
        camera.remove::<DepthPrepass>();
    }
    camera.remove::<AquaEditorCamera>();
}

fn restore_aqua_viewport_cameras(world: &mut World) {
    let cameras = world
        .query::<(Entity, &AquaEditorCamera)>()
        .iter(world)
        .map(|(entity, owned)| (entity, *owned))
        .collect::<Vec<_>>();
    for (entity, owned) in cameras {
        if let Ok(mut camera) = world.get_entity_mut(entity) {
            if owned.added_primary {
                camera.remove::<AquaPrimaryView>();
            }
            if owned.added_depth {
                camera.remove::<DepthPrepass>();
            }
            camera.remove::<AquaEditorCamera>();
        }
    }
}

#[operator(
    id = "aqua.add_ocean",
    label = "Aqua Ocean",
    description = "Add or select the scene's global Aqua ocean settings"
)]
fn aqua_add_ocean(_: In<OperatorParameters>, world: &mut World) -> OperatorResult {
    let existing = world
        .query_filtered::<Entity, With<AquaOceanSettings>>()
        .iter(world)
        .min_by_key(|entity| entity.to_bits());
    if let Some(entity) = existing {
        world.write_message(SelectionRequest::select_single(entity));
        return OperatorResult::Finished;
    }
    let entity = world
        .spawn((
            Name::new("Aqua Ocean"),
            AquaOceanSettings::default(),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    author_entity(world, entity, &[TypeId::of::<AquaOceanSettings>()]);
    OperatorResult::Finished
}

#[operator(
    id = "aqua.add_lake",
    label = "Aqua Lake",
    description = "Add a bounded circular Aqua water body"
)]
fn aqua_add_lake(_: In<OperatorParameters>, world: &mut World) -> OperatorResult {
    let entity = world
        .spawn((
            Name::new("Aqua Lake"),
            AquaWaterBodySettings::default(),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    author_entity(world, entity, &[TypeId::of::<AquaWaterBodySettings>()]);
    OperatorResult::Finished
}

#[operator(
    id = "aqua.add_river",
    label = "Aqua River",
    description = "Add a two-point flowing Aqua river"
)]
fn aqua_add_river(_: In<OperatorParameters>, world: &mut World) -> OperatorResult {
    let entity = world
        .spawn((
            Name::new("Aqua River"),
            AquaWaterBodySettings {
                shape: AquaWaterShape::River {
                    points: vec![
                        AquaRiverPoint {
                            position: Vec2::new(-8.0, 0.0),
                            width: 6.0,
                            speed: 1.5,
                        },
                        AquaRiverPoint {
                            position: Vec2::new(8.0, 0.0),
                            width: 6.0,
                            speed: 1.5,
                        },
                    ],
                },
                optics: Some(AquaOpticsPreset::ClearFresh),
            },
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    author_entity(world, entity, &[TypeId::of::<AquaWaterBodySettings>()]);
    OperatorResult::Finished
}

fn author_entity(world: &mut World, entity: Entity, components: &[TypeId]) {
    create_entity_in_ast(world, entity, None);
    for type_id in components
        .iter()
        .copied()
        .chain([TypeId::of::<Transform>(), TypeId::of::<Visibility>()])
    {
        sync_to_ast(world, entity, type_id);
    }
    world.write_message(SelectionRequest::select_single(entity));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_aqua::{WaterBody, WaterOptics, WaterShape};
    use bsn_extension::scene::document::{BsnScenePlugin, load_bsn_scene, serialize_to_bsn};

    fn register_authoring_types(app: &mut App) {
        app.register_type::<AquaWaveModel>()
            .register_type::<AquaSeaState>()
            .register_type::<AquaOpticsPreset>()
            .register_type::<AquaReflectionSettings>()
            .register_type::<AquaCausticsSettings>()
            .register_type::<AquaOceanSettings>()
            .register_type::<AquaRiverPoint>()
            .register_type::<AquaWaterShape>()
            .register_type::<AquaWaterBodySettings>();
    }

    #[test]
    fn extension_task_pool_initialization_is_idempotent() {
        initialize_extension_task_pools();
        initialize_extension_task_pools();
    }

    #[test]
    fn viewport_camera_adaptation_is_owned_and_reversible() {
        let mut app = App::new();
        app.insert_resource(EditorViewMode::Scene)
            .insert_resource(EditorViewportState::default())
            .add_systems(Update, (sync_aqua_viewport_camera, ApplyDeferred).chain());
        app.world_mut().spawn(AquaOceanSettings::default());
        let camera = app.world_mut().spawn(MainViewportCamera).id();
        app.world_mut()
            .resource_mut::<EditorViewportState>()
            .camera_entity = Some(camera);

        app.update();
        assert!(app.world().entity(camera).contains::<AquaPrimaryView>());
        assert!(app.world().entity(camera).contains::<DepthPrepass>());

        restore_aqua_viewport_cameras(app.world_mut());
        assert!(!app.world().entity(camera).contains::<AquaPrimaryView>());
        assert!(!app.world().entity(camera).contains::<DepthPrepass>());
    }

    #[test]
    fn scene_live_scene_transition_restores_only_owned_camera_components() {
        let mut app = App::new();
        let camera = app.world_mut().spawn((MainViewportCamera, DepthPrepass)).id();
        app.world_mut().spawn(AquaOceanSettings::default());
        app.insert_resource(EditorViewMode::Scene)
            .insert_resource(EditorViewportState {
                camera_entity: Some(camera),
                ..default()
            })
            .add_systems(VIEWPORT_SYNC_SCHEDULE, (sync_aqua_viewport_camera, ApplyDeferred).chain());

        app.update();
        assert!(app.world().entity(camera).contains::<AquaPrimaryView>());
        app.insert_resource(EditorViewMode::Live);
        app.update();
        assert!(!app.world().entity(camera).contains::<AquaPrimaryView>());
        assert!(app.world().entity(camera).contains::<DepthPrepass>());
        assert!(!app.world().entity(camera).contains::<AquaEditorCamera>());
        app.insert_resource(EditorViewMode::Scene);
        app.update();
        assert!(app.world().entity(camera).contains::<AquaPrimaryView>());
        assert!(app.world().entity(camera).contains::<DepthPrepass>());
    }

    #[test]
    fn repository_ocean_scene_loads_with_authored_water_and_sun() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BsnScenePlugin));
        register_authoring_types(&mut app);
        app.register_type::<DirectionalLight>();
        let source = include_str!("../../../../../assets/bsn-scenes/aqua-ocean.bsn");
        load_bsn_scene(app.world_mut(), source).expect("repository ocean scene must load");
        let mut oceans = app.world_mut().query::<(&Name, &AquaOceanSettings, &ChildOf)>();
        let (name, ocean, parent) = oceans.single(app.world()).unwrap();
        assert_eq!(name.as_str(), "Aqua Ocean");
        assert!(ocean.ocean_enabled);
        assert_eq!(ocean.level, 0.0);
        let root = parent.parent();
        let mut suns = app.world_mut().query::<(&Name, &DirectionalLight, &ChildOf)>();
        let (name, sun, parent) = suns.single(app.world()).unwrap();
        assert_eq!(name.as_str(), "Sun");
        assert_eq!(sun.illuminance, 18_000.0);
        assert_eq!(parent.parent(), root);
    }

    #[test]
    fn cleanup_preserves_preexisting_camera_components() {
        let mut world = World::new();
        let camera = world
            .spawn((
                MainViewportCamera,
                AquaPrimaryView,
                DepthPrepass,
                AquaEditorCamera {
                    added_primary: false,
                    added_depth: false,
                },
            ))
            .id();

        restore_aqua_viewport_cameras(&mut world);
        assert!(world.entity(camera).contains::<AquaPrimaryView>());
        assert!(world.entity(camera).contains::<DepthPrepass>());
        assert!(!world.entity(camera).contains::<AquaEditorCamera>());
    }

    #[test]
    fn viewport_systems_can_be_removed_during_host_update() {
        use bevy::ecs::schedule::ScheduleCleanupPolicy;

        #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
        struct ExtensionSystems;

        let mut app = App::new();
        let camera = app.world_mut().spawn(MainViewportCamera).id();
        app.world_mut().spawn(AquaOceanSettings::default());
        app.insert_resource(EditorViewportState {
            camera_entity: Some(camera),
            ..default()
        });
        app.add_systems(
            VIEWPORT_SYNC_SCHEDULE,
            (sync_aqua_viewport_camera, ApplyDeferred)
                .chain()
                .in_set(ExtensionSystems),
        );
        app.update();
        assert!(app.world().entity(camera).contains::<AquaPrimaryView>());

        // Exercise the same schedule removal primitive as BSN's lifecycle,
        // from the schedule in which the host polls completed linked builds.
        app.add_systems(Update, |world: &mut World| {
            restore_aqua_viewport_cameras(world);
            world
                .try_schedule_scope(VIEWPORT_SYNC_SCHEDULE, |world, schedule| {
                    schedule.remove_systems_in_set(
                        ExtensionSystems,
                        world,
                        ScheduleCleanupPolicy::RemoveSystemsOnly,
                    )
                })
                .expect("viewport schedule must be available during host Update")
                .expect("extension systems must be removable");
        });
        app.update();
        app.update();
        assert!(!app.world().entity(camera).contains::<AquaPrimaryView>());
        assert!(!app.world().entity(camera).contains::<DepthPrepass>());
    }

    #[test]
    fn shipped_pie_scene_has_separate_named_ocean_and_lake_entities() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BsnScenePlugin));
        register_authoring_types(&mut app);
        let source = include_str!("../../../examples/aqua-pie/assets/scene.bsn");
        load_bsn_scene(app.world_mut(), source).expect("shipped scene must load");

        let oceans = app
            .world_mut()
            .query_filtered::<(Entity, &Name), With<AquaOceanSettings>>()
            .iter(app.world())
            .map(|(entity, name)| (entity, name.as_str().to_owned()))
            .collect::<Vec<_>>();
        let lakes = app
            .world_mut()
            .query_filtered::<(Entity, &Name), With<AquaWaterBodySettings>>()
            .iter(app.world())
            .map(|(entity, name)| (entity, name.as_str().to_owned()))
            .collect::<Vec<_>>();
        assert_eq!(oceans.len(), 1);
        assert_eq!(lakes.len(), 1);
        assert_eq!(oceans[0].1, "Aqua Ocean");
        assert_eq!(lakes[0].1, "Aqua Lake");
        assert_ne!(oceans[0].0, lakes[0].0);
    }

    #[test]
    fn bsn_round_trip_persists_authoring_authority_not_runtime_projection() {
        let mut source = App::new();
        source.add_plugins((MinimalPlugins, BsnScenePlugin));
        register_authoring_types(&mut source);
        source.world_mut().spawn((
            Name::new("Aqua Ocean"),
            AquaOceanSettings::default(),
            Transform::default(),
            Visibility::default(),
        ));
        source.world_mut().spawn((
            Name::new("Aqua Lake"),
            AquaWaterBodySettings::default(),
            // Runtime projections deliberately coexist in the source world.
            // They have no ReflectComponent metadata and must not reach BSN.
            WaterBody,
            WaterShape::Circle { radius: 24.0 },
            WaterOptics::CLEAR_FRESH,
            Transform::default(),
            Visibility::default(),
        ));

        let text = serialize_to_bsn(source.world());
        assert!(text.contains("bevy_aqua_bsn::AquaOceanSettings"));
        assert!(text.contains("bevy_aqua_bsn::AquaWaterBodySettings"));
        assert!(!text.contains("bevy_aqua_core::"));
        assert!(!text.contains("bevy_aqua_sdf::"));

        let mut loaded = App::new();
        loaded.add_plugins((MinimalPlugins, BsnScenePlugin));
        register_authoring_types(&mut loaded);
        let scene = load_bsn_scene(loaded.world_mut(), &text).expect("Aqua BSN must reload");
        assert_eq!(scene.entities.len(), 2);
        assert_eq!(
            loaded
                .world_mut()
                .query::<&AquaOceanSettings>()
                .iter(loaded.world())
                .count(),
            1
        );
        assert_eq!(
            loaded
                .world_mut()
                .query::<&AquaWaterBodySettings>()
                .iter(loaded.world())
                .count(),
            1
        );
        assert_eq!(
            loaded
                .world_mut()
                .query::<&WaterBody>()
                .iter(loaded.world())
                .count(),
            0
        );
    }
}
