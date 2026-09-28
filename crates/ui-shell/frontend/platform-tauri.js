// Adaptador de escritorio. main.js sólo consume esta interfaz de plataforma;
// una compilación web podrá proporcionar otra implementación sin tocar la UI.
const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;
const { getCurrentWebview } = window.__TAURI__.webview;

window.estudioPlatform = {
  setUiZoom: (scale) => getCurrentWebview().setZoom(scale),
  newProject: () => invoke("new_project"),
  demoMidiProject: () => invoke("demo_midi_project"),
  addTrack: (kind) => invoke("add_track", { kind }),
  moveTrack: (trackId, index) => invoke("move_track", { trackId, index }),
  duplicateTrack: (trackId) => invoke("duplicate_track", { trackId }),
  removeTrack: (trackId) => invoke("remove_track", { trackId }),
  async selectAudioFile() {
    const path = await open({
      multiple: false,
      directory: false,
      title: "Importar archivo de audio",
      filters: [{ name: "Audio", extensions: ["wav", "aiff", "aif", "flac", "ogg", "opus", "mp3", "m4a"] }],
    });
    return typeof path === "string" ? path : null;
  },
  async selectStandaloneInstrument() {
    const path = await open({
      multiple: false,
      directory: false,
      title: "Elegir instrumento standalone de Wine",
      filters: [{ name: "Instrumento Windows", extensions: ["exe"] }],
    });
    return typeof path === "string" ? path : null;
  },
  async selectVst3Plugin() {
    const path = await open({
      multiple: false,
      directory: true,
      title: "Elegir bundle de instrumento VST3 (.vst3)",
    });
    return typeof path === "string" ? path : null;
  },
  async selectWinePrefix() {
    const path = await open({
      multiple: false,
      directory: true,
      title: "Elegir prefijo Wine local",
    });
    return typeof path === "string" ? path : null;
  },
  getLocalWinePrefix: (applicationPath) => invoke("get_local_wine_prefix", { applicationPath }),
  setLocalWinePrefix: (applicationPath, winePrefix) => invoke("set_local_wine_prefix", { applicationPath, winePrefix }),
  inspectVst3Plugin: (path) => invoke("inspect_vst3_plugin", { path }),
  revokeExternalCode: (path) => invoke("revoke_external_code", { path }),
  setVst3Editor: (trackId, open) => invoke("set_vst3_editor", { trackId, open }),
  launchSessionSlot: (sceneId, trackId, gridTicks, respectClipQuantization = true) => invoke("launch_session_slot", { sceneId, trackId, gridTicks, respectClipQuantization }),
  launchSessionScene: (sceneId, gridTicks) => invoke("launch_session_scene", { sceneId, gridTicks }),
  stopSessionTrack: (trackId) => invoke("stop_session_track", { trackId }),
  sessionLaunches: () => invoke("session_launches"),
  openStandaloneInstrument: ({ trackId, applicationPath }) => invoke("open_standalone_instrument", { trackId, applicationPath }),
  setTrackInstrument: (trackId, instrument) => invoke("set_track_instrument", { trackId, instrument }),
  importAudio({ path, trackId, copyIntoProject, startTick, sourceChannelSelection }) {
    return invoke("import_audio", { path, trackId, copyIntoProject, startTick, sourceChannelSelection });
  },
  editAudioRegion({ action, clipId, startTick, sourceStartSamples, durationSamples, gainDb, fadeInSamples, fadeOutSamples }) {
    return invoke("edit_audio_region", { action, clipId, startTick, sourceStartSamples, durationSamples, gainDb, fadeInSamples, fadeOutSamples });
  },
  quantizeMidiClip: (clipId, gridTicks) => invoke("quantize_midi_clip", { clipId, gridTicks }),
  moveMidiClip: (clipId, startTick) => invoke("move_midi_clip", { clipId, startTick }),
  duplicateMidiClip: (clipId) => invoke("duplicate_midi_clip", { clipId }),
  splitMidiClip: (clipId, splitTick) => invoke("split_midi_clip", { clipId, splitTick }),
  addMidiNote: ({ clipId, startTick, durationTicks, key, velocity }) => invoke("add_midi_note", { clipId, startTick, durationTicks, key, velocity }),
  updateMidiNote: (note) => invoke("update_midi_note", note),
  removeMidiNote: (note) => invoke("remove_midi_note", note),
  audioWaveform: (sourceId) => invoke("audio_waveform", { sourceId }),
  audioPreview: (sourceId) => invoke("audio_preview", { sourceId }),
  audioPreviewFile: (path) => invoke("audio_preview_file", { path }),
  inspectAudioFile: (path) => invoke("inspect_audio_file", { path }),
  async openProject() {
    const path = await open({
      multiple: false,
      directory: false,
      title: "Abrir proyecto Estudio DAW",
      filters: [{ name: "Proyecto Estudio DAW", extensions: ["json"] }],
    });
    if (typeof path !== "string") return null;
    return invoke("open_project", { path });
  },
  saveProject: () => invoke("save_project"),
  async saveProjectAs() {
    const path = await save({
      title: "Guardar proyecto Estudio DAW como",
      defaultPath: "proyecto.json",
      filters: [{ name: "Proyecto Estudio DAW", extensions: ["json"] }],
    });
    if (!path) return null;
    return invoke("save_project_as", { path });
  },
  setTransport: (command, positionTicks = null) => invoke("set_transport", { command, positionTicks }),
  projectSnapshot: () => invoke("project_snapshot"),
  setTrackMixer: (trackId, mixer) => invoke("set_track_mixer", { trackId, ...mixer }),
  setTrackIdentity: (trackId, identity) => invoke("set_track_identity", { trackId, ...identity }),
  setTrackOutput: (trackId, outputTrackId) => invoke("set_track_output", { trackId, outputTrackId }),
  setTrackInputRoute: (trackId, inputRoute) => invoke("set_track_input_route", { trackId, inputRoute }),
  setTrackRecordArm: (trackId, armed) => invoke("set_track_record_arm", { trackId, armed }),
  setTracksGroup: (trackIds, groupName) => invoke("set_tracks_group", { trackIds, groupName }),
  transportPosition: () => invoke("transport_position"),
  trackMeters: () => invoke("track_meters"),
  setLoopRange: (startTick, endTick) => invoke("set_loop_range", { startTick, endTick }),
  addScene: () => invoke("add_scene"),
  renameScene: (sceneId, name) => invoke("rename_scene", { sceneId, name }),
  removeScene: (sceneId) => invoke("remove_scene", { sceneId }),
  moveScene: (sceneId, index) => invoke("move_scene", { sceneId, index }),
  setClipSlot: (sceneId, trackId, clipKind, clipId) => invoke("set_clip_slot", { sceneId, trackId, clipKind, clipId }),
  setSessionLaunchQuantization: (sceneId, trackId, launchQuantization) => invoke("set_session_launch_quantization", { sceneId, trackId, launchQuantization }),
  setSessionLaunchMode: (sceneId, trackId, launchMode) => invoke("set_session_launch_mode", { sceneId, trackId, launchMode }),
  historyAction: (action) => invoke("history_action", { action }),
  audioRuntimeSettings: () => invoke("audio_runtime_settings"),
  audioOutputDevices: () => invoke("audio_output_devices"),
  audioInputDevices: () => invoke("audio_input_devices"),
  audioReturnDevices: () => invoke("audio_return_devices"),
  midiOutputDevices: () => invoke("midi_output_devices"),
  saveAudioSettings: (settings) => invoke("save_audio_settings", { settings }),
};
