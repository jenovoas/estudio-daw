// Adaptador exclusivo para revisar la composición del frontend desde un navegador.
// No conecta PipeWire, MIDI, filesystem, plugins ni ningún servicio remoto.
(() => {
  const STORAGE_KEY = "estudio-daw.preview.snapshot.v1";

  const clone = (value) => JSON.parse(JSON.stringify(value));
  const makeNotes = () => [
    { key: 60, startTick: 0, endTick: 480, startBeats: 0, durationBeats: 1, velocity: 104, channel: 0, noteOnIndex: 0, noteOffIndex: 1 },
    { key: 64, startTick: 480, endTick: 960, startBeats: 1, durationBeats: 1, velocity: 92, channel: 0, noteOnIndex: 2, noteOffIndex: 3 },
    { key: 67, startTick: 960, endTick: 1440, startBeats: 2, durationBeats: 1, velocity: 112, channel: 0, noteOnIndex: 4, noteOffIndex: 5 },
    { key: 72, startTick: 1440, endTick: 1920, startBeats: 3, durationBeats: 1, velocity: 98, channel: 0, noteOnIndex: 6, noteOffIndex: 7 },
  ];

  const demoSnapshot = () => ({
    projectId: "remote-preview",
    projectPath: null,
    projectRevision: 1,
    transportState: "stopped",
    audioEngineConnected: false,
    tempoBpm: 118,
    beatsPerBar: 4,
    loopRange: null,
    canUndo: false,
    canRedo: false,
    midiClipCount: 1,
    audioClipCount: 1,
    tracks: [
      {
        id: "preview-midi",
        name: "Piano principal",
        kind: "midi",
        role: "instrument",
        color: "#9b73d1",
        marker: "♫",
        annotation: "Demo visual · sin motor conectado",
        active: true,
        mute: false,
        solo: false,
        gainDb: -2,
        pan: 0,
        noteCount: 4,
        groupName: null,
        instrument: { backend: "sine" },
        outputTrackId: "preview-master",
        inputChannels: null,
        outputChannels: 2,
        recordArmed: false,
        inputRoute: null,
      },
      {
        id: "preview-audio",
        name: "Textura audio",
        kind: "audio",
        role: "audio",
        color: "#4bb68a",
        marker: "◖",
        annotation: "Región sintética para revisar la alineación visual",
        active: true,
        mute: false,
        solo: false,
        gainDb: -5,
        pan: 0,
        noteCount: 0,
        groupName: null,
        instrument: null,
        outputTrackId: "preview-master",
        inputChannels: 2,
        outputChannels: 2,
        recordArmed: false,
        inputRoute: null,
      },
      {
        id: "preview-master",
        name: "Master",
        kind: "audio",
        role: "master",
        color: "#d5a36f",
        marker: "M",
        annotation: "Salida principal informativa del preview",
        active: true,
        mute: false,
        solo: false,
        gainDb: 0,
        pan: 0,
        noteCount: 0,
        groupName: null,
        instrument: null,
        outputTrackId: null,
        inputChannels: 2,
        outputChannels: 2,
        recordArmed: false,
        inputRoute: null,
        virtualMaster: false,
      },
    ],
    midiClips: [{
      id: "preview-midi-clip",
      trackId: "preview-midi",
      name: "Melodía de prueba",
      ppq: 960,
      startTick: 0,
      startBeats: 1,
      durationBeats: 4,
      noteCount: 4,
      notes: makeNotes(),
    }],
    audioClips: [{
      id: "preview-audio-clip",
      trackId: "preview-audio",
      name: "Textura de prueba",
      sourceId: "preview-source",
      sourceDigest: "preview-source-v1",
      sourceName: "preview-audio.wav",
      sampleRateHz: 48000,
      channels: 2,
      startBeats: 5,
      durationBeats: 7,
      durationSamples: 336000,
      sourceStartSamples: 0,
      gainDb: -3,
      fadeInSamples: 9600,
      fadeOutSamples: 14400,
    }],
    scenes: [
      { id: "preview-scene-1", name: "Idea inicial" },
      { id: "preview-scene-2", name: "Textura" },
    ],
    clipSlots: [
      { sceneId: "preview-scene-1", trackId: "preview-midi", clipKind: "midi", clipId: "preview-midi-clip", launchQuantization: "global", launchMode: "loop" },
      { sceneId: "preview-scene-1", trackId: "preview-audio", clipKind: "audio", clipId: "preview-audio-clip", launchQuantization: "bar", launchMode: "loop" },
      { sceneId: "preview-scene-2", trackId: "preview-midi", clipKind: null, clipId: null, launchQuantization: "global", launchMode: "loop" },
      { sceneId: "preview-scene-2", trackId: "preview-audio", clipKind: "audio", clipId: "preview-audio-clip", launchQuantization: "global", launchMode: "one_shot" },
    ],
  });

  let snapshot = (() => {
    try {
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) || "null");
      return stored?.projectId === "remote-preview" ? stored : demoSnapshot();
    } catch {
      return demoSnapshot();
    }
  })();

  const publish = (mutate) => {
    const next = clone(snapshot);
    mutate(next);
    next.projectRevision = Number(next.projectRevision || 0) + 1;
    next.canUndo = true;
    snapshot = next;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
    } catch {
      // El preview sigue siendo funcional aunque el navegador bloquee almacenamiento.
    }
    return clone(snapshot);
  };

  const trackById = (project, trackId) => project.tracks.find((track) => track.id === trackId);
  const audioView = () => ({
    settings: {
      activeProfile: "multitrackPlayback",
      backendDeviceKey: "pipewire:default",
      liveRecord: { devicePeriodFrames: 128, playbackSafetyFrames: 512 },
      multitrackPlayback: { devicePeriodFrames: 256, playbackSafetyFrames: 1024 },
    },
    sampleRateHz: 48000,
    requestedPeriodFrames: 256,
    effectivePeriodFrames: null,
    playbackSafetyFrames: 1024,
    requestedPeriodMs: 5.33,
    effectivePeriodMs: null,
    playbackSafetyMs: 21.33,
    applyState: "preview",
  });

  const previewPlatform = {
    setUiZoom: async () => {},
    projectSnapshot: async () => clone(snapshot),
    newProject: async () => {
      snapshot = demoSnapshot();
      snapshot.tracks = [];
      snapshot.midiClips = [];
      snapshot.audioClips = [];
      snapshot.scenes = [];
      snapshot.clipSlots = [];
      snapshot.midiClipCount = 0;
      snapshot.audioClipCount = 0;
      snapshot.projectRevision = 1;
      return clone(snapshot);
    },
    demoMidiProject: async () => {
      snapshot = demoSnapshot();
      return clone(snapshot);
    },
    addTrack: async (kind) => publish((project) => {
      const id = `preview-${kind}-${Date.now()}`;
      const master = project.tracks.find((track) => track.role === "master");
      const track = {
        id,
        name: kind === "audio" ? "Pista de audio" : kind === "bus" ? "Bus" : "Pista MIDI",
        kind: kind === "bus" ? "audio" : kind,
        role: kind === "bus" ? "bus" : kind,
        color: kind === "audio" ? "#4bb68a" : kind === "bus" ? "#d5a36f" : "#9b73d1",
        marker: kind === "audio" ? "◖" : kind === "bus" ? "⇢" : "♫",
        annotation: "Pista creada en preview visual",
        active: true,
        mute: false,
        solo: false,
        gainDb: 0,
        pan: 0,
        noteCount: 0,
        groupName: null,
        instrument: kind === "midi" ? { backend: "sine" } : null,
        outputTrackId: master?.id ?? null,
        inputChannels: kind === "audio" ? 2 : null,
        outputChannels: 2,
        recordArmed: false,
        inputRoute: null,
      };
      project.tracks.splice(Math.max(0, project.tracks.length - 1), 0, track);
    }),
    moveTrack: async (trackId, index) => publish((project) => {
      const current = project.tracks.findIndex((track) => track.id === trackId);
      if (current < 0) return;
      const [track] = project.tracks.splice(current, 1);
      project.tracks.splice(Math.max(0, Math.min(index, project.tracks.length - 1)), 0, track);
    }),
    duplicateTrack: async (trackId) => publish((project) => {
      const source = trackById(project, trackId);
      if (!source || source.role === "master") return;
      const copy = clone(source);
      copy.id = `${source.id}-copy-${Date.now()}`;
      copy.name = `${source.name} copia`;
      project.tracks.splice(project.tracks.indexOf(source) + 1, 0, copy);
    }),
    removeTrack: async (trackId) => publish((project) => {
      const index = project.tracks.findIndex((track) => track.id === trackId);
      if (index < 0 || project.tracks[index].role === "master") return;
      project.tracks.splice(index, 1);
      project.midiClips = project.midiClips.filter((clip) => clip.trackId !== trackId);
      project.audioClips = project.audioClips.filter((clip) => clip.trackId !== trackId);
    }),
    setTrackMixer: async (trackId, mixer) => publish((project) => Object.assign(trackById(project, trackId) || {}, mixer)),
    setTrackEq: async (trackId, bands) => publish((project) => {
      const track = trackById(project, trackId);
      if (track) track.eqBands = clone(bands).slice(0, 8);
    }),
    setTrackIdentity: async (trackId, identity) => publish((project) => Object.assign(trackById(project, trackId) || {}, identity)),
    setTrackOutput: async (trackId, outputTrackId) => publish((project) => { const track = trackById(project, trackId); if (track) track.outputTrackId = outputTrackId; }),
    setTrackRecordArm: async (trackId, armed) => publish((project) => { const track = trackById(project, trackId); if (track) track.recordArmed = armed; }),
    setTracksGroup: async (trackIds, groupName) => publish((project) => project.tracks.forEach((track) => { if (trackIds.includes(track.id)) track.groupName = groupName || null; })),
    addScene: async () => publish((project) => project.scenes.push({ id: `preview-scene-${Date.now()}`, name: `Escena ${project.scenes.length + 1}` })),
    renameScene: async (sceneId, name) => publish((project) => { const scene = project.scenes.find((item) => item.id === sceneId); if (scene) scene.name = name; }),
    removeScene: async (sceneId) => publish((project) => { project.scenes = project.scenes.filter((scene) => scene.id !== sceneId); project.clipSlots = project.clipSlots.filter((slot) => slot.sceneId !== sceneId); }),
    moveScene: async (sceneId, index) => publish((project) => { const current = project.scenes.findIndex((scene) => scene.id === sceneId); if (current < 0) return; const [scene] = project.scenes.splice(current, 1); project.scenes.splice(Math.max(0, Math.min(index, project.scenes.length)), 0, scene); }),
    setClipSlot: async (sceneId, trackId, clipKind, clipId) => publish((project) => { const slot = project.clipSlots.find((item) => item.sceneId === sceneId && item.trackId === trackId); if (slot) Object.assign(slot, { clipKind, clipId }); }),
    setSessionLaunchQuantization: async (sceneId, trackId, launchQuantization) => publish((project) => { const slot = project.clipSlots.find((item) => item.sceneId === sceneId && item.trackId === trackId); if (slot) slot.launchQuantization = launchQuantization; }),
    setSessionLaunchMode: async (sceneId, trackId, launchMode) => publish((project) => { const slot = project.clipSlots.find((item) => item.sceneId === sceneId && item.trackId === trackId); if (slot) slot.launchMode = launchMode; }),
    setTransport: async (command) => {
      if (["play", "record"].includes(command)) throw new Error("Preview visual: el motor de audio no está conectado");
      return clone(snapshot);
    },
    transportPosition: async () => 0,
    trackMeters: async () => ({}),
    sessionLaunches: async () => [],
    launchSessionSlot: async () => { throw new Error("Preview visual: el motor de audio no está conectado"); },
    launchSessionScene: async () => { throw new Error("Preview visual: el motor de audio no está conectado"); },
    stopSessionTrack: async () => {},
    returnToArrangement: async () => clone(snapshot),
    setLoopRange: async (startTick, endTick) => publish((project) => { project.loopRange = startTick != null && endTick != null ? { startTick, endTick } : null; }),
    historyAction: async () => clone(snapshot),
    audioRuntimeSettings: async () => audioView(),
    audioOutputDevices: async () => [],
    audioInputDevices: async () => [],
    audioReturnDevices: async () => [],
    midiOutputDevices: async () => [],
    saveAudioSettings: async () => audioView(),
    audioWaveform: async () => Array.from({ length: 96 }, (_, index) => { const amplitude = 0.2 + Math.abs(Math.sin(index / 5)) * 0.75; return [-amplitude, amplitude]; }),
    audioPreview: async () => { throw new Error("La preescucha no está disponible en el preview visual"); },
    audioPreviewFile: async () => { throw new Error("La preescucha no está disponible en el preview visual"); },
    selectAudioFile: async () => null,
    inspectAudioFile: async () => { throw new Error("La selección de archivos no está disponible en el preview visual"); },
    importAudio: async () => { throw new Error("La importación de archivos no está disponible en el preview visual"); },
    createMidiClip: async ({ trackId, startTick, durationTicks, name }) => publish((project) => {
      const existing = new Set(project.midiClips.map((clip) => clip.id));
      let suffix = project.midiClips.length + 1;
      let id = `preview-midi-clip-${suffix}`;
      while (existing.has(id)) {
        suffix += 1;
        id = `preview-midi-clip-${suffix}`;
      }
      project.midiClips.push({
        id,
        trackId,
        name,
        ppq: 960,
        startTick,
        startBeats: startTick / 960,
        durationBeats: durationTicks / 960,
        noteCount: 0,
        notes: [],
      });
    }),
    quantizeMidiClip: async () => clone(snapshot),
    moveMidiClip: async () => clone(snapshot),
    duplicateMidiClip: async () => clone(snapshot),
    splitMidiClip: async () => clone(snapshot),
    addMidiNote: async () => clone(snapshot),
    updateMidiNote: async () => clone(snapshot),
    removeMidiNote: async () => clone(snapshot),
    selectVst3Plugin: async () => null,
    inspectVst3Plugin: async () => { throw new Error("La carga de plugins no está disponible en el preview visual"); },
    setTrackInstrument: async () => clone(snapshot),
    setVst3Editor: async () => { throw new Error("La GUI de plugins no está disponible en el preview visual"); },
    revokeExternalCode: async () => {},
    selectStandaloneInstrument: async () => null,
    selectWinePrefix: async () => null,
    getLocalWinePrefix: async () => null,
    setLocalWinePrefix: async () => null,
    openStandaloneInstrument: async () => { throw new Error("Los instrumentos externos no están disponibles en el preview visual"); },
    openProject: async () => null,
    saveProject: async () => { throw new Error("El preview visual no guarda proyectos"); },
    saveProjectAs: async () => { throw new Error("El preview visual no guarda proyectos"); },
  };

  window.estudioPlatform = previewPlatform;
  window.addEventListener("DOMContentLoaded", () => {
    window.setTimeout(() => document.querySelector("#demo-project")?.click(), 0);
  }, { once: true });
})();
