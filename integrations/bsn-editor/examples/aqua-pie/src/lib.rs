//! Minimal BSN project proving that editor-authored Aqua data reaches PIE.

use bevy::{core_pipeline::prepass::DepthPrepass, prelude::*};
use bevy_aqua::AquaPrimaryView;
use bevy_aqua_bsn::AquaBsnPlugin;
use bsn_runtime::BsnSceneRoot;

/// Project composition root loaded by BSN Editor's PIE runner.
#[derive(Default, Debug, Clone, Copy)]
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AquaBsnPlugin)
            .add_systems(Startup, setup_pie_world)
            .add_systems(PostStartup, setup_pie_camera)
            .add_systems(Update, setup_pie_camera);
    }
}

fn setup_pie_world(
    mut commands: Commands,
    assets: Res<AssetServer>,
    lights: Query<(), With<DirectionalLight>>,
) {
    if lights.is_empty() {
        commands.spawn((
            Name::new("Sun"),
            DirectionalLight {
                illuminance: 18_000.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.6, 0.0)),
        ));
    }
    if std::env::var_os("BSN_EDITOR_SCENE_SNAPSHOT").is_some() {
        return;
    }
    commands.spawn((
        Name::new("Aqua Scene"),
        BsnSceneRoot(assets.load("scene.bsn")),
    ));
}

fn setup_pie_camera(
    mut commands: Commands,
    cameras: Query<(Entity, Option<&Name>), (With<Camera3d>, Without<DepthPrepass>)>,
) {
    for (camera, name) in &cameras {
        let mut entity = commands.entity(camera);
        entity.insert(DepthPrepass);
        if name.is_some_and(|name| name.as_str() == "PIE Fallback Camera") {
            entity.insert((
                AquaPrimaryView,
                Transform::from_xyz(24.0, 12.0, 32.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_camera_pose_is_preserved_and_fallback_is_framed() {
        let mut app = App::new();
        app.add_systems(Update, setup_pie_camera);
        let pose = Transform::from_xyz(7.0, 8.0, 9.0);
        let authored = app.world_mut().spawn((Camera3d::default(), pose)).id();
        let fallback = app
            .world_mut()
            .spawn((Camera3d::default(), Name::new("PIE Fallback Camera")))
            .id();
        app.update();
        assert_eq!(*app.world().get::<Transform>(authored).unwrap(), pose);
        assert!(app.world().get::<DepthPrepass>(authored).is_some());
        assert_eq!(
            app.world().get::<Transform>(fallback).unwrap().translation,
            Vec3::new(24.0, 12.0, 32.0)
        );
        assert!(app.world().get::<AquaPrimaryView>(fallback).is_some());
    }
}
