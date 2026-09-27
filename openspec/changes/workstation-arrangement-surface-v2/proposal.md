# Proposal: Ableton-Inspired Workstation and Track System

## Why

La shell actual es un prototipo parcial, no una workstation terminada. Una pasada anterior marcó como completas tareas de presentación pese a no entregar el flujo de Session View, edición real ni un sistema utilizable de pistas de audio. Ableton Live 12 es la referencia principal y prioritaria de interfaz y flujo creativo. Ardour queda como consulta técnica secundaria para grabación, ruteo y señal de audio; no guía la composición visual ni la interacción. Los manuales oficiales respaldan el análisis y los requisitos verificables para Estudio DAW.

## What Changes

- Establish one versioned, undoable project model for ordered MIDI/instrument/audio/bus/master tracks, scenes, slots, sources, non-destructive playlists/regions and mixer state.
- Deliver linked Session and Arrangement surfaces over those same tracks, with real clip/scene launch, arrangement editing, contextual MIDI/audio editing, browser/media import, and mixer operations backed end to end.
- Provide audio tracks with real source provenance, channels, input/process/pan/gain/meter/output signal flow, import, playback, and a complete recording/monitoring workflow before enabling those controls.
- Create a discoverable command/menu/shortcut system covering Ableton Live 12's menu families and workflows, with further Estudio DAW capabilities from the project vision; stage delivery without marking deferred commands complete or presenting them as active.
- Follow Live 12's creative hierarchy, Session/Arrangement relationship, Browser, clip launching, contextual editing and integrated mixer. Consult Ardour only for audio-engineering details absent or underspecified in Ableton; do not inherit Ardour's visual hierarchy or editing flow. Do not copy proprietary branding or assets.
- Keep every capability honest: domain edits use typed commands, reversible operations have undo/redo, platform/media work stays in adapters/workers, and the realtime callback remains bounded and allocation-free.
- Migrate existing projects additively and prove existing track/clip identities, media provenance and musical positions survive load/save.

## Capabilities

### New Capabilities

### Modified Capabilities

- `desktop-workstation-ui`: ampliar el contrato de composición de DAW, controles visibles, arreglo y estados de sesión.

## Impact

Afecta `project-model`, `command-bus`, `application`, `ui-shell` y el runtime de audio; requiere migraciones compatibles, comandos tipados, scheduler de clips, audio media workers y una matriz rastreable entre menús/opciones y operaciones. La UI queda abierta hasta que cada superficie y ruta anunciada esté implementada y verificada.
