use anyhow::Result;
use bevy::{
    asset::AssetLoader,
    ecs::{FromResources, Resources},
    prelude::*,
    property::{property_serde::DynamicPropertiesDeserializer, PropertyTypeRegistry},
    type_registry::TypeRegistry,
};
use parking_lot::RwLock;
use serde::de::DeserializeSeed;
use std::{path::Path, sync::Arc};

pub struct DynamicPropertiesLoader {
    property_type_registry: Arc<RwLock<PropertyTypeRegistry>>,
}

impl FromResources for DynamicPropertiesLoader {
    fn from_resources(resources: &Resources) -> Self {
        let type_registry = resources.get::<TypeRegistry>().unwrap();
        DynamicPropertiesLoader {
            property_type_registry: type_registry.property.clone(),
        }
    }
}

impl AssetLoader<DynamicProperties> for DynamicPropertiesLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<DynamicProperties> {
        let registry = self.property_type_registry.read();
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
        let dynamic_properties_deserializer = DynamicPropertiesDeserializer::new(&registry);
        let dynamic_properties = dynamic_properties_deserializer.deserialize(&mut deserializer)?;
        Ok(dynamic_properties)
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["prp"];
        EXTENSIONS
    }
}

#[derive(Debug, Default, Properties)]
pub struct MyStruct {
    pub data: usize,
    pub v_s: SomeType,
}

#[derive(Debug, Default, Properties)]
pub struct SomeType {
    pub v: Vec<usize>,
    pub s: String,
}

#[derive(Debug, Default)]
pub struct PropTest {
    pub my_struct: MyStruct,
    pub dynamic_properties: Handle<DynamicProperties>,
}

pub struct PropTestPlugin;

impl Plugin for PropTestPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<DynamicProperties>()
            .add_asset_loader::<DynamicProperties, DynamicPropertiesLoader>()
            .register_property::<SomeType>()
            .register_property::<MyStruct>()
            .init_resource::<PropTest>()
            .add_startup_system(prop_test_init.system())
            .add_system(prop_test_changed.system());
    }
}

fn prop_test_init(asset_server: Res<AssetServer>, mut prop_test: ResMut<PropTest>) {
    println!("Init: {:#?}", prop_test.my_struct);
    prop_test.dynamic_properties = asset_server
        .load("assets/properties/prop_test.prp")
        .unwrap();
    asset_server.watch_for_changes().unwrap();
}

fn prop_test_changed(
    mut prop_test: ResMut<PropTest>,
    props: ChangedRes<Assets<DynamicProperties>>,
) {
    let dynamic_properties = props.get(&prop_test.dynamic_properties).unwrap();
    prop_test.my_struct.apply(dynamic_properties);
    println!("Asset hot-loaded: {:#?}", prop_test.my_struct);
}
