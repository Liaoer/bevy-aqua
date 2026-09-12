# bevy-aqua for BSN Editor

This isolated workspace packages Aqua as a directory-installed BSN Editor
extension targeting ABI v7 and the `bsn-sdk` v0.6.5 contract. It deliberately
stays outside the main Aqua workspace so ordinary Bevy builds never resolve an
editor SDK dependency.

The three layers are:

- `bevy-aqua-bsn`: reflected scene-authoring components and their projection
  into Aqua's runtime resources/components;
- `bevy-aqua-bsn-editor`: reversible editor operators, inspector routing, and
  primary-viewport adaptation through `bsn_extension` only;
- `bevy-aqua-bsn-extension`: the package wrapper consumed by the editor's
  generated ABI-v7 shim.

Open this workspace in BSN Editor or build/package the extension with the
editor's extension CLI. Enabling it for the first time requires an editor
restart because Aqua installs App- and RenderApp-lifetime infrastructure.

## Linked-source development

BSN Editor builds and installs development generations directly from source.
Open `File > Extensions...`, choose `Link from source...`, and select:

```text
crates/bevy-aqua-bsn-extension
```

The extension declares `activation = "startup_required"` and
`bootstrap_compatibility = 3`. Restart the editor after the first successful
linked build so Aqua's App- and RenderApp-lifetime infrastructure can
bootstrap. After that, changes to reversible editor code (`register`,
`unregister`, operators, inspectors, menus, and systems) build and activate
automatically while `Hot Reload: On` and the bootstrap compatibility number is
unchanged.

The removable viewport adapter runs in `PreUpdate`: the host processes linked
build completion in `Update`, and cannot remove systems from that same active
schedule. When upgrading an older Aqua generation that registered its adapter
in `Update`, restart to replace that generation before testing live reload.
The native host must also preserve registration-time system-set handles across
DLL generations. Re-interning an equal owner in the executable can fail with
`SetNotFound`; the companion BSN Editor fix keeps canonical handles in the
World. Rebuild the host and its linked extensions together when updating that
SDK implementation.

Increment `bootstrap_compatibility` whenever `bootstrap` changes or when a
type or asset consumed by bootstrap changes. That generation is staged for the
next editor restart instead of being partially activated.

Compatibility 3 also waits for the first extracted wave, foam, and cascade
resources before preparing their GPU bind groups. The editor may render before
main-world startup/extraction; that interval must skip preparation rather than
report missing-resource errors. The waves, foam, and query crates each cover
this state with `prepare_before_first_extraction_is_skipped`.

## Local ABI-v7 build

When the extension uses the contracted Git SDK while the development editor
comes from a local checkout, enable BSN's exact external-source redirect before
calling the host:

```powershell
$env:BSN_EDITOR_EXTERNAL_SDK_SOURCE = `
  'git+https://github.com/Liaoer/bsn-sdk.git?tag=v0.6.5'

& $BsnEditor build-extension `
  crates/bevy-aqua-bsn-extension `
  --profile debug `
  --out ../../artifacts/bsn-extension
```

The host and its static SDK must come from the same Cargo profile and toolchain.
The generated SDK plan should redirect the following edges to host artifacts:

```text
bevy_aqua_bsn_editor -> bsn_extension
bsn_extension -> bsn_api
bsn_api -> bsn_api_internal
```

Pack an already verified DLL with an isolated publisher key:

```powershell
& $BsnEditor extension keygen <publisher-key-path>
& $BsnEditor extension pack `
  --project crates/bevy-aqua-bsn-extension `
  --library <verified-dll> `
  --key <publisher-key-path> `
  --out bevy_aqua.editor-0.1.2-x86_64-pc-windows-msvc.bsnext
```

Standalone BSN games that load authored Aqua components add `AquaBsnPlugin`
to their normal game composition root. The editor extension installs the same
plugin during its `StartupRequired` bootstrap for scene preview.

## Play the active scene from the repository root

Open the Aqua repository root in BSN Editor. Its `bsn_editor.toml` delegates
code compilation to the isolated `examples/aqua-pie` project:

```toml
manifest_path = "integrations/bsn-editor/examples/aqua-pie/Cargo.toml"
package = "bevy-aqua-bsn-pie"
plugin = "GamePlugin"
scene_source = "active"
```

Open a `.bsn` scene (for example `assets/bsn-scenes/aqua-ocean.bsn` or
`assets/bsn-scenes/aqua-lake.bsn`) and press Play. The editor captures the active
scene document, including unsaved component edits and inline assets, at the
moment Play is requested. It passes an immutable temporary snapshot to the
runner; it never saves over the scene file. Assets remain rooted in the opened
Aqua project, while Cargo dependencies and patches belong to the isolated PIE
workspace. The Game panel displays the rendered runtime; Stop discards that
runtime and returns to the editor document.

While PIE runs, switch to Scene view and open/select another scene tab. Use
**Tools > Load Current Scene in PIE** to replace the running scene with that
document's current contents. This keeps the process and persistent game
infrastructure alive. Game code can also load scenes through the updated
runtime's `SceneLoadRequest(asset_server.load("levels/next.bsn"))` message.
Invalid scene syntax is rejected before removing the previous managed scene.

The generic runner supplies a fallback camera only when a scene has no camera.
Aqua preserves authored camera transforms and adds the depth prepass needed
for water. The example's fixed `scene.bsn` fallback is used only outside the
editor's active-scene mode. Existing projects that omit `scene_source` retain
their project-owned entry behavior.

This workflow requires the companion host/runtime changes for delegated
manifests and scene snapshots. Rebuild the host and linked extensions together.
The Git runtime pin remains the standalone source contract; SDK-driven builds
redirect it to the exact updated host runtime.

For an empty document, use Tools > Toggle command palette > Aqua Ocean.
Repeating Aqua Ocean selects the existing global ocean. Its settings are in
the Aqua inspector category. Names containing spaces use `#"Aqua Ocean"` in
BSN; sibling entities need comma-separated children under a shared root.
