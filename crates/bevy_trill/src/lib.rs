use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::LazyLock,
};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{
    Asset, AssetApp, AssetLoader, AssetServer, Assets, Handle, LoadContext, io::Reader,
};
use bevy_ecs::{
    entity::Entity,
    event::EntityEvent,
    message::{Message, Messages},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
    world::{Mut, World},
};
use bevy_mod_props::{Props, PropsMutExt, Registry};
use bevy_reflect::TypePath;
use thiserror::Error;
use trill::{core::engine::ResponseEngine, script::ScriptCompiler};

pub use trill::*;
use ustr::{Ustr, UstrMap};

pub struct TrillPlugin;

impl Plugin for TrillPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EngineState>()
            .init_asset::<TrillFile>()
            .init_asset_loader::<TrillFileLoader>()
            .add_message::<RequestResponse>()
            .add_message::<LoadResponseEngine>()
            .add_systems(PostUpdate, (load_engine, manage_responses).chain());
    }
}

#[derive(Asset, TypePath)]
pub struct TrillFile {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Error)]
pub enum TrillFileError {
    #[error("io error loading trill file: {0}")]
    Io(#[from] std::io::Error),
    #[error("trill file not valid utf8: {0}")]
    NonUTF8(#[from] std::string::FromUtf8Error),
}

#[derive(Default)]
struct TrillFileLoader;

impl AssetLoader for TrillFileLoader {
    type Asset = TrillFile;
    type Settings = ();
    type Error = TrillFileError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let name = format!("{}", load_context.path().file_stem().unwrap().display());
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let source = String::from_utf8(bytes)?;
        Ok(TrillFile { name, source })
    }

    fn extensions(&self) -> &[&str] {
        &["trill"]
    }
}

#[derive(Message)]
pub struct LoadResponseEngine {
    partition_variables: Vec<Ustr>,
    sources: Vec<TrillSource>,
}

impl Default for LoadResponseEngine {
    fn default() -> Self {
        LoadResponseEngine {
            partition_variables: vec![
                Ustr::from("concept"),
                Ustr::from("name"),
                Ustr::from("class"),
            ],
            sources: vec![],
        }
    }
}

impl LoadResponseEngine {
    pub fn add_partition(mut self, variable: impl Into<Ustr>) -> Self {
        self.partition_variables.push(variable.into());
        self
    }

    pub fn add_source(mut self, source: TrillSource) -> Self {
        self.sources.push(source);
        self
    }

    pub fn add_source_asset(self, handle: Handle<TrillFile>) -> Self {
        self.add_source(TrillSource::Handle(handle))
    }

    pub fn add_source_string(self, name: String, source: String) -> Self {
        self.add_source(TrillSource::InMemory(TrillFile { name, source }))
    }

    pub fn add_source_path(self, path: impl Into<PathBuf>) -> Self {
        self.add_source(TrillSource::File(path.into()))
    }
}

pub enum TrillSource {
    Handle(Handle<TrillFile>),
    InMemory(TrillFile),
    File(PathBuf),
}

#[derive(Resource, Default)]
pub enum EngineState {
    #[default]
    UnLoaded,
    Loading {
        partition_variables: Vec<Ustr>,
        files: Vec<Handle<TrillFile>>,
    },
    Loaded(ResponseEngine),
    LoadFailed,
}

fn load_engine(
    trill_files: Res<Assets<TrillFile>>,
    asset_server: Res<AssetServer>,
    mut engine_state: ResMut<EngineState>,
    mut load_messages: ResMut<Messages<LoadResponseEngine>>,
) {
    if let Some(message) = load_messages.drain().last() {
        let LoadResponseEngine {
            partition_variables,
            sources,
        } = message;
        let files: Vec<_> = sources
            .into_iter()
            .map(|s| match s {
                TrillSource::Handle(handle) => handle,
                TrillSource::InMemory(trill_file) => asset_server.add(trill_file),
                TrillSource::File(path) => asset_server.load(path),
            })
            .collect();
        *engine_state = EngineState::Loading {
            partition_variables,
            files,
        };
    }

    if let EngineState::Loading {
        partition_variables,
        files,
    } = &*engine_state
    {
        let files = files
            .iter()
            .map(|s| trill_files.get(s))
            .collect::<Option<Vec<_>>>();
        if let Some(files) = files {
            let mut compiler = ScriptCompiler::new();
            for file in files {
                compiler.add_module(&file.name, &file.source);
            }
            for var in partition_variables {
                compiler.add_partition_variable(*var);
            }
            let (engine, report) = compiler.compile();
            report.print();
            *engine_state = match engine {
                Some(engine) => EngineState::Loaded(engine),
                None => EngineState::LoadFailed,
            }
        }
    }
}

static CONCEPT: LazyLock<Ustr> = LazyLock::new(|| Ustr::from("concept"));

#[derive(Message)]
pub struct RequestResponse {
    entity: Entity,
    props: Props,
}

impl RequestResponse {
    pub fn new(entity: Entity, concept: impl AsRef<str>) -> RequestResponse {
        RequestResponse {
            entity,
            props: Props::new().with(*CONCEPT, concept.as_ref()),
        }
    }
}

impl Deref for RequestResponse {
    type Target = Props;

    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl DerefMut for RequestResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.props
    }
}

#[derive(EntityEvent)]
pub struct Response {
    entity: Entity,
    properties: UstrMap<String>,
}

impl Response {
    pub fn get(&self, key: impl Into<Ustr>) -> Option<&str> {
        self.properties.get(&key.into()).map(|s| s.as_str())
    }
}

pub fn manage_responses(world: &mut World) {
    world.resource_scope(|world, mut engine_state: Mut<EngineState>| {
        let EngineState::Loaded(engine) = &mut *engine_state else {
            return;
        };

        world.get_resource_or_init::<Props>();
        world.resource_scope(|world, world_props: Mut<Props>| {
            let world_props = world_props.into_inner();
            world.get_resource_or_init::<Registry>();
            world.resource_scope(|world, registry: Mut<Registry>| {
                world.resource_scope(|world, mut requests: Mut<Messages<RequestResponse>>| {
                    for mut request in requests.drain() {
                        let mut entity = world.entity_mut(request.entity);
                        let charicter_props = entity.props_mut();

                        let registration = registry.lookup_entity(request.entity);
                        if let Some(name) = registration.name {
                            request.props.set("name", name);
                        }
                        if let Some(class) = registration.class {
                            request.props.set("class", class);
                        }

                        let mut rng = rand::rng();
                        if let Some(properties) = engine.find_best_response(
                            &request.props,
                            charicter_props,
                            world_props,
                            &mut rng,
                        ) {
                            world.trigger(Response {
                                entity: request.entity,
                                properties: properties.clone(),
                            });
                        }
                    }
                })
            })
        })
    });
}
