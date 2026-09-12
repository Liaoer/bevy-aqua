//! Directory-installable ABI-v7 Aqua authoring extension.

use bevy_aqua_bsn_editor::AquaEditorExtension;
use bsn_extension::prelude::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct AquaExtension;

impl BsnExtension for AquaExtension {
    fn id(&self) -> String {
        AquaEditorExtension.id()
    }

    fn label(&self) -> String {
        AquaEditorExtension.label()
    }

    fn description(&self) -> String {
        AquaEditorExtension.description()
    }

    fn bootstrap_requirement(&self) -> ExtensionBootstrapRequirement {
        AquaEditorExtension.bootstrap_requirement()
    }

    fn import_reflect_inventory(&self) -> bool {
        AquaEditorExtension.import_reflect_inventory()
    }

    fn bootstrap(&self, ctx: &mut ExtensionBootstrapContext) -> ExtensionResult {
        AquaEditorExtension.bootstrap(ctx)
    }

    fn register(&self, ctx: &mut ExtensionContext) -> ExtensionResult {
        AquaEditorExtension.register(ctx)
    }

    fn unregister(&self, ctx: &mut ExtensionUnregisterContext) -> ExtensionResult {
        AquaEditorExtension.unregister(ctx)
    }
}
