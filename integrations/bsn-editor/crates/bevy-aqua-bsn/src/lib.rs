//! Reflected BSN scene-authoring data projected into Aqua's runtime facade.
//!
//! The components in this crate are the serialized authority. Generated Aqua
//! resources and components are projections and are never written back to BSN.

use bevy::prelude::*;
use bevy_aqua::{
    AquaPlugin, AquaSettings, Caustics, Ocean, OceanWaves, ReflectionMode, RiverPath, RiverPoint,
    SeaState, WaterBody, WaterOptics, WaterShape, WaveModel,
};

/// Ordered projection stage for hosts that need to run before or after Aqua's
/// scene-authoring bridge.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AquaBsnSet {
    ProjectAuthoring,
}

/// Installs Aqua and projects reflected authoring components into its runtime
/// resources and components.
#[derive(Default, Debug, Clone, Copy)]
pub struct AquaBsnPlugin;

impl Plugin for AquaBsnPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<AquaPlugin>() {
            app.add_plugins(AquaPlugin);
        }
        app.register_type::<AquaWaveModel>()
            .register_type::<AquaSeaState>()
            .register_type::<AquaOpticsPreset>()
            .register_type::<AquaReflectionSettings>()
            .register_type::<AquaCausticsSettings>()
            .register_type::<AquaOceanSettings>()
            .register_type::<AquaRiverPoint>()
            .register_type::<AquaWaterShape>()
            .register_type::<AquaWaterBodySettings>()
            .init_resource::<OceanProjectionState>()
            .configure_sets(PreUpdate, AquaBsnSet::ProjectAuthoring)
            .add_systems(
                PreUpdate,
                (
                    project_ocean_settings,
                    project_changed_water_bodies,
                    remove_orphaned_water_body_projections,
                    ApplyDeferred,
                )
                    .chain()
                    .in_set(AquaBsnSet::ProjectAuthoring),
            );
    }
}

#[derive(Reflect, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AquaWaveModel {
    #[default]
    Analytic,
    Spectral,
}

impl From<AquaWaveModel> for WaveModel {
    fn from(value: AquaWaveModel) -> Self {
        match value {
            AquaWaveModel::Analytic => Self::Analytic,
            AquaWaveModel::Spectral => Self::Spectral,
        }
    }
}

#[derive(Reflect, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AquaSeaState {
    Calm,
    #[default]
    Moderate,
    Rough,
}

impl From<AquaSeaState> for SeaState {
    fn from(value: AquaSeaState) -> Self {
        match value {
            AquaSeaState::Calm => Self::Calm,
            AquaSeaState::Moderate => Self::Moderate,
            AquaSeaState::Rough => Self::Rough,
        }
    }
}

#[derive(Reflect, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AquaOpticsPreset {
    #[default]
    DeepOcean,
    Coastal,
    Tropical,
    ClearFresh,
}

impl AquaOpticsPreset {
    fn runtime(self) -> WaterOptics {
        match self {
            Self::DeepOcean => WaterOptics::DEEP_OCEAN,
            Self::Coastal => WaterOptics::COASTAL,
            Self::Tropical => WaterOptics::TROPICAL,
            Self::ClearFresh => WaterOptics::CLEAR_FRESH,
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
pub enum AquaReflectionSettings {
    Cubemap,
    Planar { scale: f32, distortion: f32 },
}

impl Default for AquaReflectionSettings {
    fn default() -> Self {
        Self::Planar {
            scale: 0.5,
            distortion: 0.02,
        }
    }
}

impl From<AquaReflectionSettings> for ReflectionMode {
    fn from(value: AquaReflectionSettings) -> Self {
        match value {
            AquaReflectionSettings::Cubemap => Self::Cubemap,
            AquaReflectionSettings::Planar { scale, distortion } => {
                Self::Planar { scale, distortion }
            }
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
pub struct AquaCausticsSettings {
    pub strength: f32,
    pub scale: f32,
    pub speed: f32,
    pub depth_max: f32,
}

impl Default for AquaCausticsSettings {
    fn default() -> Self {
        let value = Caustics::default();
        Self {
            strength: value.strength,
            scale: value.scale,
            speed: value.speed,
            depth_max: value.depth_max,
        }
    }
}

impl From<AquaCausticsSettings> for Caustics {
    fn from(value: AquaCausticsSettings) -> Self {
        Self {
            strength: value.strength,
            scale: value.scale,
            speed: value.speed,
            depth_max: value.depth_max,
        }
    }
}

/// One scene-authored global Aqua environment. Exactly one is authoritative;
/// when several exist the lowest entity id wins deterministically.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct AquaOceanSettings {
    pub ocean_enabled: bool,
    pub level: f32,
    pub wave_model: AquaWaveModel,
    pub sea_state: AquaSeaState,
    pub shallow_water_attenuation: f32,
    pub wind_direction_degrees: f32,
    pub wind_speed: f32,
    pub fetch: f32,
    pub flow: Vec2,
    pub detail_strength: f32,
    pub optics: AquaOpticsPreset,
    pub atmospheric_sunlight: bool,
    pub far_tier_start: f32,
    pub far_tier_end: f32,
    pub reflections: AquaReflectionSettings,
    pub caustics: Option<AquaCausticsSettings>,
}

impl Default for AquaOceanSettings {
    fn default() -> Self {
        let waves = OceanWaves::default();
        let appearance = AquaSettings::default();
        Self {
            ocean_enabled: true,
            level: 0.0,
            wave_model: AquaWaveModel::Analytic,
            sea_state: AquaSeaState::Moderate,
            shallow_water_attenuation: waves.shallow_water_attenuation,
            wind_direction_degrees: waves.wind_direction_degrees,
            wind_speed: waves.wind_speed,
            fetch: waves.fetch,
            flow: waves.flow,
            detail_strength: appearance.detail_strength,
            optics: AquaOpticsPreset::DeepOcean,
            atmospheric_sunlight: appearance.atmospheric_sunlight,
            far_tier_start: appearance.far_tier_start,
            far_tier_end: appearance.far_tier_end,
            reflections: AquaReflectionSettings::default(),
            caustics: appearance.caustics.map(|value| AquaCausticsSettings {
                strength: value.strength,
                scale: value.scale,
                speed: value.speed,
                depth_max: value.depth_max,
            }),
        }
    }
}

impl AquaOceanSettings {
    fn waves(&self) -> OceanWaves {
        OceanWaves {
            model: self.wave_model.into(),
            sea_state: self.sea_state.into(),
            shallow_water_attenuation: self.shallow_water_attenuation,
            wind_direction_degrees: self.wind_direction_degrees,
            wind_speed: self.wind_speed,
            fetch: self.fetch,
            flow: self.flow,
        }
    }

    fn appearance(&self) -> AquaSettings {
        AquaSettings {
            detail_strength: self.detail_strength,
            water_optics: self.optics.runtime(),
            atmospheric_sunlight: self.atmospheric_sunlight,
            far_tier_start: self.far_tier_start,
            far_tier_end: self.far_tier_end,
            reflections: self.reflections.into(),
            caustics: self.caustics.map(Into::into),
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
pub struct AquaRiverPoint {
    pub position: Vec2,
    pub width: f32,
    pub speed: f32,
}

impl Default for AquaRiverPoint {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            width: 4.0,
            speed: 1.0,
        }
    }
}

impl From<AquaRiverPoint> for RiverPoint {
    fn from(value: AquaRiverPoint) -> Self {
        RiverPoint::new(value.position, value.width, value.speed)
    }
}

#[derive(Reflect, Clone, Debug, PartialEq)]
pub enum AquaWaterShape {
    Circle {
        radius: f32,
    },
    Polygon {
        points: Vec<Vec2>,
    },
    River {
        points: Vec<AquaRiverPoint>,
    },
    Corridor {
        points: Vec<AquaRiverPoint>,
        width: f32,
    },
}

impl Default for AquaWaterShape {
    fn default() -> Self {
        Self::Circle { radius: 24.0 }
    }
}

impl AquaWaterShape {
    fn runtime(&self) -> WaterShape {
        match self {
            Self::Circle { radius } => WaterShape::Circle { radius: *radius },
            Self::Polygon { points } => WaterShape::Polygon {
                points: points.clone(),
            },
            Self::River { points } => WaterShape::River {
                path: RiverPath {
                    points: points.iter().copied().map(Into::into).collect(),
                },
            },
            Self::Corridor { points, width } => WaterShape::Corridor {
                path: RiverPath {
                    points: points.iter().copied().map(Into::into).collect(),
                },
                width: *width,
            },
        }
    }
}

/// One localized water surface authored in BSN and projected onto the same
/// entity as Aqua's runtime `WaterBody`, `WaterShape`, and optional optics.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
#[require(Transform)]
pub struct AquaWaterBodySettings {
    pub shape: AquaWaterShape,
    pub optics: Option<AquaOpticsPreset>,
}

impl Default for AquaWaterBodySettings {
    fn default() -> Self {
        Self {
            shape: AquaWaterShape::default(),
            optics: Some(AquaOpticsPreset::ClearFresh),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct ProjectedWaterBody;

#[derive(Resource, Default)]
struct OceanProjectionState {
    source: Option<Entity>,
    last: Option<AquaOceanSettings>,
    warned_multiple: bool,
}

fn project_ocean_settings(
    authoring: Query<(Entity, &AquaOceanSettings)>,
    mut state: ResMut<OceanProjectionState>,
    mut waves: ResMut<OceanWaves>,
    mut appearance: ResMut<AquaSettings>,
    mut commands: Commands,
) {
    let mut selected: Option<(Entity, &AquaOceanSettings)> = None;
    let mut count = 0usize;
    for candidate in &authoring {
        count += 1;
        if selected.is_none_or(|current| candidate.0.to_bits() < current.0.to_bits()) {
            selected = Some(candidate);
        }
    }

    if count > 1 && !state.warned_multiple {
        warn!(
            "multiple AquaOceanSettings components are authored; the lowest entity id is authoritative"
        );
    }
    state.warned_multiple = count > 1;

    let Some((entity, settings)) = selected else {
        if state.source.take().is_some() {
            state.last = None;
            *waves = OceanWaves::default();
            *appearance = AquaSettings::default();
            commands.remove_resource::<Ocean>();
        }
        return;
    };

    if state.source == Some(entity) && state.last.as_ref() == Some(settings) {
        return;
    }
    *waves = settings.waves();
    *appearance = settings.appearance();
    if settings.ocean_enabled {
        commands.insert_resource(Ocean {
            level: settings.level,
        });
    } else {
        commands.remove_resource::<Ocean>();
    }
    state.source = Some(entity);
    state.last = Some(settings.clone());
}

fn project_changed_water_bodies(
    changed: Query<(Entity, &AquaWaterBodySettings), Changed<AquaWaterBodySettings>>,
    mut commands: Commands,
) {
    for (entity, settings) in &changed {
        let mut target = commands.entity(entity);
        target.insert((WaterBody, settings.shape.runtime(), ProjectedWaterBody));
        if let Some(optics) = settings.optics {
            target.insert(optics.runtime());
        } else {
            target.remove::<WaterOptics>();
        }
    }
}

fn remove_orphaned_water_body_projections(
    mut removed: RemovedComponents<AquaWaterBodySettings>,
    projected: Query<(), With<ProjectedWaterBody>>,
    mut commands: Commands,
) {
    for entity in removed.read() {
        if projected.get(entity).is_ok() {
            commands
                .entity(entity)
                .remove::<(WaterBody, WaterShape, WaterOptics, ProjectedWaterBody)>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<OceanWaves>()
            .init_resource::<AquaSettings>()
            .init_resource::<OceanProjectionState>()
            .add_systems(
                Update,
                (
                    project_ocean_settings,
                    project_changed_water_bodies,
                    remove_orphaned_water_body_projections,
                    ApplyDeferred,
                )
                    .chain(),
            );
        app
    }

    #[test]
    fn authored_ocean_projects_resources_and_can_disable_unbounded_water() {
        let mut app = projection_app();
        let mut authored = AquaOceanSettings::default();
        authored.level = 7.5;
        authored.sea_state = AquaSeaState::Rough;
        let entity = app.world_mut().spawn(authored).id();

        app.update();
        assert_eq!(app.world().resource::<Ocean>().level, 7.5);
        assert_eq!(
            app.world().resource::<OceanWaves>().sea_state,
            SeaState::Rough
        );

        app.world_mut()
            .get_mut::<AquaOceanSettings>(entity)
            .unwrap()
            .ocean_enabled = false;
        app.update();
        assert!(!app.world().contains_resource::<Ocean>());
    }

    #[test]
    fn localized_authoring_projects_and_cleans_only_its_runtime_components() {
        let mut app = projection_app();
        let entity = app.world_mut().spawn(AquaWaterBodySettings::default()).id();

        app.update();
        assert!(app.world().entity(entity).contains::<WaterBody>());
        assert!(app.world().entity(entity).contains::<WaterShape>());
        assert!(app.world().entity(entity).contains::<WaterOptics>());

        app.world_mut()
            .entity_mut(entity)
            .remove::<AquaWaterBodySettings>();
        app.update();
        assert!(!app.world().entity(entity).contains::<WaterBody>());
        assert!(!app.world().entity(entity).contains::<WaterShape>());
        assert!(!app.world().entity(entity).contains::<WaterOptics>());
    }
}
