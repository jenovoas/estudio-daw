const platform = window.estudioPlatform;

const elements = {
  open: document.querySelector("#open-project"),
  browserOpen: document.querySelector("#browser-open"),
  importAudio: document.querySelector("#browser-import-audio"),
  importPreview: document.querySelector("#audio-import-preview"),
  importCommit: document.querySelector("#audio-import-commit"),
  importSelected: document.querySelector("#audio-import-selected"),
  importTrack: document.querySelector("#audio-import-track"),
  importBar: document.querySelector("#audio-import-bar"),
  editCursorPosition: document.querySelector("#edit-cursor-position"),
  importMode: document.querySelector("#audio-import-mode"),
  importChannels: document.querySelector("#audio-import-channels"),
  browserDemo: document.querySelector("#browser-demo"),
  newProject: document.querySelector("#new-project"),
  demoProject: document.querySelector("#demo-project"),
  addMidiTrack: document.querySelector("#add-midi-track"),
  addAudioTrack: document.querySelector("#add-audio-track"),
  save: document.querySelector("#save-project"),
  saveAs: document.querySelector("#save-project-as"),
  path: document.querySelector("#project-path"),
  name: document.querySelector("#project-name"),
  browserName: document.querySelector("#browser-project-name"),
  tempo: document.querySelector("#tempo"),
  transportTempo: document.querySelector("#transport-tempo"),
  transport: document.querySelector("#transport-state"),
  transportPosition: document.querySelector("#transport-position"),
  loopRangeReadout: document.querySelector("#loop-range-readout"),
  loopPointA: document.querySelector("#loop-point-a"),
  loopPointB: document.querySelector("#loop-point-b"),
  loopRangeClear: document.querySelector("#loop-range-clear"),
  tracks: document.querySelector("#track-list"),
  lanes: document.querySelector("#arrangement-lanes"),
  ruler: document.querySelector("#timeline-ruler"),
  arrangementView: document.querySelector(".arrangement-scroll"),
  sessionView: document.querySelector("#session-view"),
  mixerView: document.querySelector("#mixer-view"),
  arrangementLegend: document.querySelector("#arrangement-legend"),
  showArrangement: document.querySelector("#show-arrangement"),
  showSession: document.querySelector("#show-session"),
  showMixer: document.querySelector("#show-mixer"),
  trackCount: document.querySelector("#track-count"),
  midiCount: document.querySelector("#midi-count"),
  audioCount: document.querySelector("#audio-count"),
  revision: document.querySelector("#revision"),
  projectStatus: document.querySelector("#project-status"),
  engine: document.querySelector("#engine-status"),
  noticeTitle: document.querySelector("#notice-title"),
  noticeText: document.querySelector("#notice-text"),
  play: document.querySelector("#play"),
  pause: document.querySelector("#pause"),
  stop: document.querySelector("#stop"),
  panic: document.querySelector("#panic"),
  metronome: document.querySelector("#metronome"),
  undo: document.querySelector("#undo"),
  redo: document.querySelector("#redo"),
  audioProfile: document.querySelector("#audio-profile"),
  devicePeriod: document.querySelector("#device-period"),
  playbackSafety: document.querySelector("#playback-safety"),
  saveAudioSettings: document.querySelector("#save-audio-settings"),
  audioApplyState: document.querySelector("#audio-apply-state"),
  audioSampleRate: document.querySelector("#audio-sample-rate"),
  audioRequestedPeriod: document.querySelector("#audio-requested-period"),
  audioEffectivePeriod: document.querySelector("#audio-effective-period"),
  audioSafetyDuration: document.querySelector("#audio-safety-duration"),
};

let hasProject = false;
let audioSettings = null;
let projectTransportState = "stopped";
let transportPositionTick = 0;
let transportPositionPollPending = false;
let transportLoopErrorReported = false;
let metronomeEnabled = false;
let loopRange = null;
let pendingLoopStartTick = null;
let pendingAudioPath = null;
let editCursorTick = 0;
let editCursorProjectId = null;
const waveformCache = new Map();
let previewContext = null;
let currentPreview = null;

function selectedAudioProfile() {
  return audioSettings?.[elements.audioProfile.value];
}

function renderAudioProfile(view = null) {
  if (!audioSettings) return;
  const profile = selectedAudioProfile();
  elements.devicePeriod.value = profile.devicePeriodFrames;
  elements.playbackSafety.value = profile.playbackSafetyFrames;
  if (view) {
    const rate = view.sampleRateHz;
    elements.audioSampleRate.textContent = `${rate.toLocaleString()} Hz`;
    elements.audioRequestedPeriod.textContent = `${view.requestedPeriodFrames} frames (${view.requestedPeriodMs.toFixed(2)} ms)`;
    elements.audioEffectivePeriod.textContent = view.effectivePeriodFrames == null
      ? "No disponible"
      : `${view.effectivePeriodFrames} frames (${(view.effectivePeriodFrames * 1000 / rate).toFixed(2)} ms)`;
    elements.audioSafetyDuration.textContent = `${view.playbackSafetyFrames} frames (${view.playbackSafetyMs.toFixed(2)} ms)`;
    elements.audioApplyState.textContent = view.applyState === "nextStream" ? "Pendiente · próximo inicio" : view.applyState;
  } else {
    const rate = 48_000;
    const requested = Number(profile.devicePeriodFrames);
    const safety = Number(profile.playbackSafetyFrames);
    elements.audioSampleRate.textContent = `${rate.toLocaleString()} Hz`;
    elements.audioRequestedPeriod.textContent = `${requested} frames (${(requested * 1000 / rate).toFixed(2)} ms)`;
    elements.audioEffectivePeriod.textContent = "No disponible";
    elements.audioSafetyDuration.textContent = `${safety} frames (${(safety * 1000 / rate).toFixed(2)} ms)`;
  }
}

async function loadAudioSettings() {
  try {
    const view = await platform.audioRuntimeSettings();
    audioSettings = view.settings;
    elements.audioProfile.value = audioSettings.activeProfile;
    renderAudioProfile(view);
  } catch (error) {
    elements.audioApplyState.textContent = "No se pudo cargar";
    setNotice("Configuración de audio no disponible", String(error));
  }
}

function setNotice(title, text) {
  elements.noticeTitle.textContent = title;
  elements.noticeText.textContent = text;
}

async function whileBusy(buttons, operation) {
  const previous = buttons.map((button) => button.disabled);
  for (const button of buttons) {
    button.disabled = true;
    button.setAttribute("aria-busy", "true");
  }
  try {
    return await operation();
  } finally {
    buttons.forEach((button, index) => {
      button.disabled = previous[index];
      button.removeAttribute("aria-busy");
    });
  }
}

function setProjectEnabled(enabled) {
  hasProject = enabled;
  for (const button of [elements.save, elements.saveAs, elements.play, elements.pause, elements.stop, elements.addMidiTrack, elements.addAudioTrack]) {
    button.disabled = !enabled;
    if (!enabled) button.title = `${button.getAttribute("aria-label") ?? "Acción"}: abre o crea un proyecto primero`;
  }
  for (const button of [elements.loopPointA, elements.loopPointB, elements.loopRangeClear]) {
    button.disabled = !enabled;
  }
  elements.metronome.disabled = !enabled;
  elements.panic.disabled = !enabled || !["playing", "paused"].includes(projectTransportState);
  for (const control of [elements.importAudio, elements.importTrack, elements.importBar, elements.importMode, elements.importChannels]) {
    control.disabled = !enabled;
  }
  elements.importPreview.disabled = !enabled || !pendingAudioPath;
  elements.importCommit.disabled = !enabled || !pendingAudioPath;
}

function selectSurface(surface) {
  const selected = {
    arrangement: elements.showArrangement,
    session: elements.showSession,
    mixer: elements.showMixer,
  };
  elements.arrangementView.hidden = surface !== "arrangement";
  elements.sessionView.hidden = surface !== "session";
  elements.mixerView.hidden = surface !== "mixer";
  elements.arrangementLegend.hidden = surface !== "arrangement";
  for (const [name, button] of Object.entries(selected)) {
    const active = name === surface;
    button.classList.toggle("is-selected", active);
    button.setAttribute("aria-selected", String(active));
  }
}

function trackChannelDescription(track) {
  const input = track.inputChannels == null ? "sin entrada asignada" : `${track.inputChannels} canales de entrada`;
  return `${input} · ${track.outputChannels} canales de salida`;
}

function trackOutputDescription(track, tracks) {
  if (track.role === "master") return "Salida física: configuración de plataforma pendiente";
  const target = tracks.find((candidate) => candidate.id === track.outputTrackId);
  return target ? `Salida interna → ${target.name}` : "Sin salida interna asignada";
}

function renderSessionSurface(tracks) {
  elements.sessionView.replaceChildren();
  const heading = document.createElement("div");
  heading.className = "surface-heading";
  heading.innerHTML = "<strong>SESSION</strong><span>Identidad compartida · lanzamiento de clips pendiente</span>";
  elements.sessionView.append(heading);
  if (tracks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "surface-empty";
    empty.textContent = "El proyecto todavía no tiene pistas.";
    elements.sessionView.append(empty);
    return;
  }
  const grid = document.createElement("div");
  grid.className = "session-track-grid";
  for (const track of tracks) {
    const column = document.createElement("article");
    column.className = "session-track-card";
    column.style.setProperty("--track-color", track.color);
    const name = document.createElement("strong");
    name.textContent = track.name;
    const type = document.createElement("span");
    type.textContent = track.role === "master" ? "MASTER" : track.kind.toUpperCase();
    const channels = document.createElement("small");
    channels.textContent = trackChannelDescription(track);
    const emptySlot = document.createElement("div");
    emptySlot.className = "session-empty-slot";
    emptySlot.textContent = "Sin escena";
    column.append(name, type, channels, emptySlot);
    grid.append(column);
  }
  elements.sessionView.append(grid);
}

function renderMixerSurface(tracks) {
  elements.mixerView.replaceChildren();
  const heading = document.createElement("div");
  heading.className = "surface-heading";
  heading.innerHTML = "<strong>MEZCLADOR</strong><span>Controles de pista aplicados a la reproducción</span>";
  elements.mixerView.append(heading);
  if (tracks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "surface-empty";
    empty.textContent = "El proyecto todavía no tiene canales.";
    elements.mixerView.append(empty);
    return;
  }
  const channels = document.createElement("div");
  channels.className = "mixer-channel-list";
  for (const track of tracks) {
    const channel = document.createElement("article");
    channel.className = "mixer-channel";
    channel.style.setProperty("--track-color", track.color);
    const title = document.createElement("strong");
    title.textContent = track.name;
    const role = document.createElement("span");
    role.className = "mixer-role";
    role.textContent = track.role === "master" ? "MASTER" : track.kind.toUpperCase();
    const routing = document.createElement("small");
    routing.textContent = track.role === "master"
      ? trackOutputDescription(track, tracks)
      : `${trackChannelDescription(track)} · ${trackOutputDescription(track, tracks)}`;
    const mix = document.createElement("span");
    mix.className = "mixer-values";
    mix.textContent = `${Number(track.gainDb).toFixed(1)} dB · Pan ${Number(track.pan).toFixed(2)}${track.mute ? " · Silencio" : ""}${track.solo ? " · Solo" : ""}${track.active ? "" : " · Inactiva"}`;
    channel.append(title, role, routing, mix);
    if (track.role !== "master") {
      const controls = document.createElement("div");
      controls.className = "mixer-controls";
      const toggle = (label, property, value) => {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "mixer-toggle";
        button.textContent = label;
        button.title = `${label}: ${track.name}`;
        button.setAttribute("aria-label", `${label} ${track.name}`);
        button.setAttribute("aria-pressed", String(value));
        button.classList.toggle("is-selected", value);
        button.addEventListener("click", () => updateTrackMixer(track, { [property]: !value }, `${label} actualizado`));
        controls.append(button);
      };
      toggle("ACT", "active", track.active);
      toggle("M", "mute", track.mute);
      toggle("S", "solo", track.solo);

      const slider = (label, property, value, min, max, step, suffix = "") => {
        const field = document.createElement("label");
        field.className = "mixer-slider";
        const caption = document.createElement("span");
        caption.textContent = `${label} ${Number(value).toFixed(property === "pan" ? 2 : 1)}${suffix}`;
        const input = document.createElement("input");
        input.type = "range";
        input.min = String(min);
        input.max = String(max);
        input.step = String(step);
        input.value = String(value);
        input.setAttribute("aria-label", `${label} de ${track.name}`);
        input.addEventListener("change", () => updateTrackMixer(track, { [property]: Number(input.value) }, `${label} actualizado`));
        field.append(caption, input);
        controls.append(field);
      };
      slider("Ganancia", "gainDb", track.gainDb, -60, 12, 0.5, " dB");
      slider("Panorama", "pan", track.pan, -1, 1, 0.05);
      channel.append(controls);
    }
    channels.append(channel);
  }
  elements.mixerView.append(channels);
}

function updateTrackMixer(track, changes, title) {
  const mixer = {
    active: track.active,
    mute: track.mute,
    solo: track.solo,
    gainDb: Number(track.gainDb),
    pan: Number(track.pan),
    ...changes,
  };
  return runCommand(title, () => platform.setTrackMixer(track.id, mixer));
}

function renderSnapshot(snapshot) {
  setProjectEnabled(true);
  if (snapshot.projectId !== editCursorProjectId) {
    editCursorTick = 0;
    transportPositionTick = 0;
    editCursorProjectId = snapshot.projectId;
  }
  projectTransportState = snapshot.transportState;
  elements.panic.disabled = !snapshot.audioEngineConnected || !["playing", "paused"].includes(projectTransportState);
  loopRange = snapshot.loopRange ?? null;
  if (!loopRange) pendingLoopStartTick = null;
  elements.loopRangeReadout.textContent = loopRange
    ? `A ${formatBarBeat(loopRange.startTick)} · B ${formatBarBeat(loopRange.endTick)}`
    : pendingLoopStartTick === null ? "Sin rango" : `A ${formatBarBeat(pendingLoopStartTick)} · fija B`;
  if (projectTransportState === "stopped") renderTransportPosition(0);
  elements.save.disabled = !snapshot.projectPath;
  elements.save.title = snapshot.projectPath ? "Guardar proyecto" : "Guarda como para elegir una ubicación";
  elements.path.textContent = snapshot.projectPath ?? "Proyecto sin ruta";
  elements.path.title = snapshot.projectPath ?? "";
  const projectLabel = snapshot.projectPath?.split(/[\\/]/).at(-1) ?? snapshot.projectId;
  elements.name.textContent = projectLabel;
  elements.browserName.textContent = projectLabel;
  elements.tempo.textContent = Number(snapshot.tempoBpm).toFixed(1);
  elements.transportTempo.textContent = Number(snapshot.tempoBpm).toFixed(1);
  elements.transport.textContent = snapshot.transportState.toUpperCase();
  document.querySelector(".transport-bar").dataset.state = snapshot.transportState.toLowerCase();
  elements.trackCount.textContent = snapshot.trackCount;
  elements.midiCount.textContent = snapshot.midiClipCount;
  elements.audioCount.textContent = snapshot.audioClipCount;
  elements.revision.textContent = `REV ${snapshot.projectRevision}`;
  elements.projectStatus.textContent = snapshot.projectPath ? "PROYECTO ABIERTO" : "PROYECTO SIN GUARDAR";
  elements.engine.textContent = snapshot.audioEngineConnected
    ? "Core listo · motor de audio conectado"
    : "Core listo · motor de audio aún no conectado";
  elements.undo.disabled = !snapshot.canUndo;
  elements.redo.disabled = !snapshot.canRedo;
  renderSessionSurface(snapshot.tracks);
  renderMixerSurface(snapshot.tracks);
  const previousTrack = elements.importTrack.value;
  elements.importTrack.replaceChildren();
  const audioTracks = snapshot.tracks.filter((track) => track.role === "audio");
  for (const track of audioTracks) {
    const option = document.createElement("option");
    option.value = track.id;
    option.textContent = track.name;
    elements.importTrack.append(option);
  }
  elements.importTrack.disabled = audioTracks.length === 0;
  elements.importAudio.disabled = false;
  if (audioTracks.some((track) => track.id === previousTrack)) elements.importTrack.value = previousTrack;
  elements.importCommit.disabled = !pendingAudioPath || audioTracks.length === 0;

  elements.tracks.replaceChildren();
  elements.lanes.replaceChildren();
  renderTimelineRuler(snapshot.beatsPerBar || 4);
  if (snapshot.tracks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = "El proyecto todavía no tiene pistas.";
    elements.tracks.append(empty);
    const emptyLane = document.createElement("div");
    emptyLane.className = "empty-state timeline-empty";
    emptyLane.textContent = "Sin pistas en el arreglo";
    elements.lanes.append(emptyLane);
    renderEditCursor();
    renderTransportPosition(transportPositionTick);
    return;
  }

  for (const [trackIndex, track] of snapshot.tracks.entries()) {
    const row = document.createElement("div");
    row.className = "track-row";
    row.style.setProperty("--track-color", track.color);
    const icon = document.createElement("span");
    icon.className = `track-icon ${track.kind}`;
    icon.textContent = track.role === "master" ? "M" : track.kind === "audio" ? "◖" : "♫";
    const label = document.createElement("span");
    label.textContent = track.name;
    const details = document.createElement("span");
    details.className = "track-meta";
    const mixState = [track.mute ? "MUTE" : null, track.solo ? "SOLO" : null, !track.active ? "OFF" : null].filter(Boolean).join(" · ");
    details.textContent = `${track.role === "master" ? "MASTER" : track.kind === "audio" ? "AUDIO" : "MIDI"}${track.kind === "midi" ? ` · ${track.noteCount} notas` : track.kind === "audio" ? ` · ${track.outputChannels} ch` : ""}${mixState ? ` · ${mixState}` : ""}`;
    const name = document.createElement("div");
    name.className = "track-name";
    name.append(icon, label);
    row.append(name, details);
    elements.tracks.append(row);

    const lane = document.createElement("div");
    lane.className = "timeline-lane";
    lane.style.setProperty("--track-color", track.color);
    lane.addEventListener("click", (event) => {
      if (event.target.closest(".audio-clip, .midi-clip, button")) return;
      setEditCursorFromX(event.clientX, lane.getBoundingClientRect());
    });
    const clips = snapshot.midiClips.filter((clip) => clip.trackId === track.id);
    for (const clip of clips) {
      const block = document.createElement("div");
      block.className = "midi-clip";
      block.style.setProperty("--clip-hue", String((trackIndex * 54 + 24) % 360));
      block.title = `${clip.name} · ${clip.noteCount} notas`;
      const left = Math.max(0, Number(clip.startBeats) || 0);
      const width = Math.max(0.25, Number(clip.durationBeats) || 0.25);
      block.style.left = `${left / (snapshot.beatsPerBar * 16) * 100}%`;
      block.style.width = `${Math.min(width / (snapshot.beatsPerBar * 16) * 100, 100)}%`;
      const clipLabel = document.createElement("span");
      clipLabel.className = "clip-label";
      clipLabel.textContent = clip.name;
      block.append(clipLabel);
      const noteLayer = document.createElement("div");
      noteLayer.className = "clip-note-layer";
      for (const note of clip.notes) {
        const noteMark = document.createElement("span");
        noteMark.className = "clip-note";
        const noteLeft = Math.max(0, Number(note.startBeats) || 0);
        const noteWidth = Math.max(0.04, Number(note.durationBeats) || 0.04);
        noteMark.style.left = `${noteLeft / width * 100}%`;
        noteMark.style.width = `${Math.min(noteWidth / width * 100, 100)}%`;
        noteMark.style.bottom = `${4 + Math.max(0, Math.min(1, (note.key - 48) / 36)) * 30}px`;
        noteMark.style.opacity = String(0.5 + Math.max(0, Math.min(127, note.velocity)) / 254);
        noteLayer.append(noteMark);
      }
      block.append(noteLayer);
      lane.append(block);
    }
    const audioClips = (snapshot.audioClips ?? []).filter((clip) => clip.trackId === track.id);
    for (const clip of audioClips) {
      const block = document.createElement("div");
      block.className = "audio-clip";
      block.title = `${clip.name} · ${clip.sourceName ?? "fuente"} · ${clip.sampleRateHz} Hz · ${clip.channels} canales · ${clip.durationBeats.toFixed(2)} pulsos. Arrastra para mover; usa los bordes para recortar.`;
      const left = Math.max(0, Number(clip.startBeats) || 0);
      const width = Math.max(0.25, Number(clip.durationBeats) || 0.25);
      block.style.left = `${left / (snapshot.beatsPerBar * 16) * 100}%`;
      block.style.width = `${Math.min(width / (snapshot.beatsPerBar * 16) * 100, 100)}%`;
      const label = document.createElement("span");
      label.className = "clip-label";
      label.textContent = clip.name;
      const preview = document.createElement("button");
      preview.type = "button";
      preview.className = "audio-preview-button";
      preview.setAttribute("aria-label", `Preescuchar ${clip.name}`);
      preview.title = "Preescucha aislada · hasta 30 segundos";
      preview.textContent = "▶";
      preview.addEventListener("click", () => previewAudio(clip.sourceId, preview));
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "audio-region-remove";
      remove.setAttribute("aria-label", `Borrar región ${clip.name}`);
      remove.title = "Quitar región del proyecto; conservar archivo fuente";
      remove.textContent = "×";
      remove.addEventListener("click", async () => {
        await whileBusy([remove], async () => {
          try {
            const snapshot = await platform.editAudioRegion({ action: "remove", clipId: clip.id });
            renderSnapshot(snapshot);
            const playbackNote = projectTransportState === "playing"
              ? "El plan activo se reconstruyó; la fuente de audio se conservó."
              : projectTransportState === "paused"
                ? "La fuente se conservó; el plan actualizado se aplicará al reanudar."
                : "La fuente de audio se conservó.";
            setNotice("Región quitada", playbackNote);
          } catch (error) {
            setNotice("No se pudo quitar la región", String(error));
          }
        });
      });
      const leftHandle = makeAudioTrimHandle("left", clip.name);
      const rightHandle = makeAudioTrimHandle("right", clip.name);
      const wave = document.createElement("div");
      wave.className = "audio-waveform";
      block.append(label, remove, preview, leftHandle, rightHandle, wave);
      bindAudioRegionEditing(block, lane, clip, snapshot.beatsPerBar, snapshot.tempoBpm);
      lane.append(block);
      if (clip.sourceId) loadWaveform(clip.sourceId, clip.sourceDigest, wave);
    }
    if (clips.length === 0 && audioClips.length === 0) {
      const empty = document.createElement("span");
      empty.className = "lane-empty-label";
      empty.textContent = "Sin clips";
      lane.append(empty);
    }
    const cursor = document.createElement("span");
    cursor.className = "edit-cursor";
    cursor.setAttribute("aria-hidden", "true");
    lane.append(cursor);
    const playhead = document.createElement("span");
    playhead.className = "playhead";
    playhead.setAttribute("aria-hidden", "true");
    lane.append(playhead);
    elements.lanes.append(lane);
  }
  renderEditCursor();
  renderTransportPosition(transportPositionTick);
}

function makeAudioTrimHandle(edge, name) {
  const handle = document.createElement("button");
  handle.type = "button";
  handle.className = `audio-region-handle audio-region-handle-${edge}`;
  handle.dataset.edge = edge;
  handle.setAttribute("aria-label", `Recortar ${edge === "left" ? "inicio" : "final"} de ${name}`);
  handle.title = "Arrastra hacia dentro para recortar sin modificar el archivo";
  return handle;
}

function bindAudioRegionEditing(block, lane, clip, beatsPerBar, tempoBpm) {
  const timelineBeats = Math.max(1, Number(beatsPerBar) || 4) * 16;
  const originalLeft = Math.max(0, Number(clip.startBeats) || 0);
  const originalWidth = Math.max(0.25, Number(clip.durationBeats) || 0.25);
  let gesture = null;

  block.addEventListener("pointerdown", (event) => {
    const handle = event.target.closest(".audio-region-handle");
    if (event.target.closest("button") && !handle) return;
    if (event.button !== 0) return;
    const bounds = lane.getBoundingClientRect();
    if (bounds.width <= 0) return;
    gesture = {
      pointerId: event.pointerId,
      edge: handle?.dataset.edge ?? "move",
      startX: event.clientX,
      laneWidth: bounds.width,
    };
    block.setPointerCapture(event.pointerId);
    block.classList.add("is-editing");
    event.preventDefault();
  });

  block.addEventListener("pointermove", (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const delta = event.clientX - gesture.startX;
    const deltaPercent = delta / gesture.laneWidth * 100;
    if (gesture.edge === "move") {
      const originalLeftPercent = originalLeft / timelineBeats * 100;
      const widthPercent = originalWidth / timelineBeats * 100;
      block.style.left = `${Math.max(0, Math.min(100 - widthPercent, originalLeftPercent + deltaPercent))}%`;
    } else if (gesture.edge === "left") {
      const trim = Math.max(0, Math.min(deltaPercent, originalWidth / timelineBeats * 100 - 0.3));
      block.style.left = `${originalLeft / timelineBeats * 100 + trim}%`;
      block.style.width = `${originalWidth / timelineBeats * 100 - trim}%`;
    } else {
      const trim = Math.min(0, Math.max(deltaPercent, -originalWidth / timelineBeats * 100 + 0.3));
      block.style.width = `${originalWidth / timelineBeats * 100 + trim}%`;
    }
  });

  const finish = async (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const current = gesture;
    gesture = null;
    block.classList.remove("is-editing");
    if (event.type === "pointercancel") {
      block.style.left = `${originalLeft / timelineBeats * 100}%`;
      block.style.width = `${originalWidth / timelineBeats * 100}%`;
      return;
    }
    const delta = event.clientX - current.startX;
    if (Math.abs(delta) < 2) {
      block.style.left = `${originalLeft / timelineBeats * 100}%`;
      block.style.width = `${originalWidth / timelineBeats * 100}%`;
      return;
    }
    const deltaBeats = delta / current.laneWidth * timelineBeats;
    const samplesPerBeat = clip.sampleRateHz * 60 / Math.max(1, Number(tempoBpm) || 120);
    let edit;
    if (current.edge === "move") {
      edit = {
        action: "move",
        clipId: clip.id,
        startTick: Math.round(Math.max(0, Math.min(timelineBeats - originalWidth, originalLeft + deltaBeats)) * 480),
      };
    } else {
      const trimBeats = current.edge === "left" ? Math.max(0, deltaBeats) : Math.max(0, -deltaBeats);
      if (trimBeats === 0) {
        block.style.left = `${originalLeft / timelineBeats * 100}%`;
        block.style.width = `${originalWidth / timelineBeats * 100}%`;
        return;
      }
      const trimSamples = Math.max(0, Math.round(trimBeats * samplesPerBeat));
      const boundedSamples = Math.min(trimSamples, Math.max(0, clip.durationSamples - 1));
      if (current.edge === "left") {
        const appliedBeats = boundedSamples / samplesPerBeat;
        edit = {
          action: "trim",
          clipId: clip.id,
          startTick: Math.round((originalLeft + appliedBeats) * 480),
          sourceStartSamples: clip.sourceStartSamples + boundedSamples,
          durationSamples: clip.durationSamples - boundedSamples,
        };
      } else {
        edit = {
          action: "trim",
          clipId: clip.id,
          sourceStartSamples: clip.sourceStartSamples,
          durationSamples: clip.durationSamples - boundedSamples,
        };
      }
    }
    try {
      const snapshot = await platform.editAudioRegion(edit);
      renderSnapshot(snapshot);
      const playbackNote = projectTransportState === "stopped"
        ? "La fuente permanece intacta; puedes deshacer el cambio desde el historial."
        : projectTransportState === "playing"
          ? "La fuente permanece intacta; el plan activo se reconstruyó en un límite de bloque."
          : "La fuente permanece intacta; el plan actualizado se aplicará al reanudar.";
      setNotice("Región actualizada", playbackNote);
    } catch (error) {
      try {
        renderSnapshot(await platform.projectSnapshot());
      } catch {
        // La vista actual queda como referencia si el refresco también falla.
      }
      setNotice("No se pudo editar la región", String(error));
    }
  };
  block.addEventListener("pointerup", finish);
  block.addEventListener("pointercancel", finish);
}

function stopPreview() {
  currentPreview?.stop();
  currentPreview = null;
}

async function previewAudio(sourceId, button) {
  if (!sourceId) return;
  if (projectTransportState === "playing") {
    setNotice("Preescucha no disponible", "Detén o pausa el transporte antes de escuchar un fragmento aislado.");
    return;
  }
  button.disabled = true;
  try {
    stopPreview();
    const encoded = await platform.audioPreview(sourceId);
    await playPreviewBytes(encoded);
    setNotice("Preescucha", "Fragmento aislado de hasta 30 segundos; no mueve el transporte del proyecto.");
  } catch (error) {
    setNotice("No se pudo preescuchar el audio", String(error));
  } finally {
    button.disabled = false;
  }
}

async function playPreviewBytes(encoded) {
  if (!previewContext) previewContext = new AudioContext();
  const binary = atob(encoded);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  const audioBuffer = await previewContext.decodeAudioData(bytes.buffer);
  await previewContext.resume();
  const source = previewContext.createBufferSource();
  source.buffer = audioBuffer;
  source.connect(previewContext.destination);
  currentPreview = source;
  source.addEventListener("ended", () => {
    if (currentPreview === source) currentPreview = null;
  }, { once: true });
  source.start();
}

async function previewPendingAudio() {
  if (!pendingAudioPath) return;
  if (projectTransportState === "playing") {
    setNotice("Preescucha no disponible", "Detén o pausa el transporte antes de escuchar el archivo seleccionado.");
    return;
  }
  elements.importPreview.disabled = true;
  try {
    stopPreview();
    await playPreviewBytes(await platform.audioPreviewFile(pendingAudioPath));
    setNotice("Preescucha del archivo", "Fragmento Opus de hasta 30 segundos. El archivo aún no se ha importado.");
  } catch (error) {
    setNotice("No se pudo preescuchar el archivo", String(error));
  } finally {
    elements.importPreview.disabled = !pendingAudioPath;
  }
}

elements.importAudio.addEventListener("click", async () => {
  try {
    stopPreview();
    const path = await platform.selectAudioFile();
    if (!path) return;
    pendingAudioPath = null;
    elements.importPreview.disabled = true;
    elements.importCommit.disabled = true;
    const metadata = await platform.inspectAudioFile(path);
    pendingAudioPath = path;
    const channelOptions = [];
    if (metadata.channels <= 2) {
      channelOptions.push({ label: metadata.channels === 1 ? "Canal 1 · mono" : "Canales 1–2 · estéreo", value: Array.from({ length: metadata.channels }, (_, index) => index) });
    } else {
      for (let channel = 0; channel < metadata.channels; channel += 1) {
        channelOptions.push({ label: `Canal ${channel + 1} · mono`, value: [channel] });
        if (channel + 1 < metadata.channels) channelOptions.push({ label: `Canales ${channel + 1}–${channel + 2} · estéreo`, value: [channel, channel + 1] });
      }
    }
    elements.importChannels.replaceChildren(...channelOptions.map((option) => {
      const item = document.createElement("option");
      item.value = option.value.join(",");
      item.textContent = option.label;
      return item;
    }));
    elements.importChannels.disabled = false;
    const fileName = path.split(/[\\/]/).at(-1) ?? path;
    elements.importSelected.textContent = `${fileName} · ${metadata.sampleRateHz.toLocaleString()} Hz · ${metadata.channels} ch · ${metadata.durationSeconds.toFixed(2)} s`;
    elements.importPreview.disabled = false;
    elements.importCommit.disabled = !hasProject || !elements.importTrack.value;
    setNotice("Archivo listo para revisar", "Puedes escuchar un fragmento antes de decidir si lo importas.");
  } catch (error) {
    setNotice("No se pudo inspeccionar el archivo", String(error));
  }
});

elements.importPreview.addEventListener("click", previewPendingAudio);
elements.importTrack.addEventListener("change", () => {
  elements.importCommit.disabled = !pendingAudioPath || !elements.importTrack.value;
});
elements.importCommit.addEventListener("click", async () => {
  const trackId = elements.importTrack.value;
  if (!pendingAudioPath || !trackId) return;
  stopPreview();
  await whileBusy([elements.importCommit], async () => {
    try {
      const snapshot = await platform.importAudio({
        path: pendingAudioPath,
        trackId,
        copyIntoProject: elements.importMode.value === "copy",
        sourceChannelSelection: elements.importChannels.value.split(",").map(Number),
        startTick: editCursorTick,
      });
      pendingAudioPath = null;
      elements.importSelected.textContent = "No hay archivo seleccionado";
      elements.importPreview.disabled = true;
      renderSnapshot(snapshot);
      setNotice("Audio importado", "La fuente y región se registraron; el Arreglo muestra su waveform. La reproducción de sesión sigue pendiente.");
    } catch (error) {
      setNotice("No se pudo importar audio", String(error));
    }
  });
});

async function loadWaveform(sourceId, sourceDigest, container) {
  try {
    const cacheKey = sourceDigest ?? sourceId;
    let bins = waveformCache.get(cacheKey);
    if (!bins) {
      bins = platform.audioWaveform(sourceId);
      waveformCache.set(cacheKey, bins);
    }
    bins = await bins;
    const points = bins.map(([min, max], index) => {
      const x = bins.length <= 1 ? 0 : index * 512 / (bins.length - 1);
      return [x, Math.max(0, Math.min(100, 50 * (1 - max))), Math.max(0, Math.min(100, 50 * (1 - min)))];
    });
    const top = points.map(([x, y]) => `${x},${y}`).join(" ");
    const bottom = [...points].reverse().map(([x, , y]) => `${x},${y}`).join(" ");
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 512 100");
    svg.setAttribute("preserveAspectRatio", "none");
    const shape = document.createElementNS("http://www.w3.org/2000/svg", "polygon");
    shape.setAttribute("points", `${top} ${bottom}`);
    svg.append(shape);
    container.replaceChildren(svg);
  } catch (error) {
    waveformCache.delete(sourceDigest ?? sourceId);
    container.title = `No se pudo calcular la forma de onda: ${String(error)}`;
  }
}

function renderTimelineRuler(beatsPerBar) {
  elements.ruler.replaceChildren();
  for (let bar = 1; bar <= 16; bar += 1) {
    const tick = document.createElement("span");
    tick.className = "bar-tick";
    tick.textContent = String(bar);
    elements.ruler.append(tick);
  }
  const cursor = document.createElement("span");
  cursor.className = "edit-cursor";
  cursor.setAttribute("aria-hidden", "true");
  elements.ruler.append(cursor);
  const playhead = document.createElement("span");
  playhead.className = "playhead";
  playhead.setAttribute("aria-hidden", "true");
  elements.ruler.append(playhead);
  elements.ruler.dataset.beatsPerBar = String(beatsPerBar);
  renderEditCursor();
}

function renderTransportPosition(ticks) {
  transportPositionTick = Math.max(0, Number(ticks) || 0);
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const ticksPerBar = beatsPerBar * 960;
  const timelineTicks = ticksPerBar * 16;
  const left = `${Math.max(0, Math.min(100, transportPositionTick / timelineTicks * 100))}%`;
  document.querySelectorAll(".playhead").forEach((playhead) => { playhead.style.left = left; });
  const bar = Math.floor(transportPositionTick / ticksPerBar) + 1;
  const beatPosition = (transportPositionTick % ticksPerBar) / 960;
  const beat = Math.floor(beatPosition) + 1;
  const subdivision = Math.floor((beatPosition % 1) * 4) + 1;
  elements.transportPosition.textContent = `${bar}.${beat}.${subdivision}`;
}

function formatBarBeat(ticks) {
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const ticksPerBar = beatsPerBar * 960;
  const bar = Math.floor(ticks / ticksPerBar) + 1;
  const beat = (ticks % ticksPerBar) / 960 + 1;
  return `${bar}.${beat.toLocaleString("es-CL", { maximumFractionDigits: 2 })}`;
}

function renderEditCursor() {
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const timelineTicks = beatsPerBar * 16 * 480;
  const left = `${Math.max(0, Math.min(100, editCursorTick / timelineTicks * 100))}%`;
  document.querySelectorAll(".edit-cursor").forEach((cursor) => { cursor.style.left = left; });
  const bar = Math.floor(editCursorTick / (beatsPerBar * 480)) + 1;
  const beat = ((editCursorTick % (beatsPerBar * 480)) / 480) + 1;
  elements.editCursorPosition.textContent = `Cursor: compás ${bar} · pulso ${beat.toLocaleString("es-CL", { maximumFractionDigits: 2 })}`;
  elements.importBar.value = String(bar);
}

function setEditCursorFromX(clientX, bounds) {
  if (bounds.width <= 0) return;
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const timelineTicks = beatsPerBar * 16 * 480;
  const fraction = Math.max(0, Math.min(1, (clientX - bounds.left) / bounds.width));
  editCursorTick = Math.round(fraction * timelineTicks / 120) * 120;
  renderEditCursor();
}

async function runCommand(title, operation) {
  try {
    const snapshot = await operation();
    if (!snapshot) return;
    renderSnapshot(snapshot);
    setNotice(title, "Cambios aplicados al estado del proyecto.");
  } catch (error) {
    setNotice("La operación falló", String(error));
  }
}

elements.open.addEventListener("click", async () => {
  await whileBusy([elements.open, elements.browserOpen], async () => { try {
    const snapshot = await platform.openProject();
    if (!snapshot) return;
    renderSnapshot(snapshot);
    setNotice("Proyecto abierto", "El estado se carga a través de ProjectApplication.");
  } catch (error) {
    setNotice("No se pudo abrir el proyecto", String(error));
  } });
});

async function addTrack(kind, button) {
  await whileBusy([button], async () => { try {
    const snapshot = await platform.addTrack(kind);
    renderSnapshot(snapshot);
    setNotice(`${kind === "audio" ? "Pista de audio" : "Pista MIDI"} creada`, "La pista quedó en el proyecto y su creación puede deshacerse desde el historial.");
  } catch (error) {
    setNotice("No se pudo crear la pista", String(error));
  } });
}

elements.addMidiTrack.addEventListener("click", () => addTrack("midi", elements.addMidiTrack));
elements.addAudioTrack.addEventListener("click", () => addTrack("audio", elements.addAudioTrack));
elements.ruler.addEventListener("click", async (event) => {
  setEditCursorFromX(event.clientX, elements.ruler.getBoundingClientRect());
  if (projectTransportState !== "playing") return;
  try {
    const snapshot = await platform.setTransport("seek", editCursorTick * 2);
    renderSnapshot(snapshot);
    renderTransportPosition(await platform.transportPosition());
    setNotice("Transporte reubicado", `Reproducción desde ${elements.transportPosition.textContent}.`);
  } catch (error) {
    setNotice("No se pudo reubicar el transporte", String(error));
  }
});
elements.loopPointA.addEventListener("click", () => {
  pendingLoopStartTick = editCursorTick * 2;
  if (loopRange && pendingLoopStartTick < loopRange.endTick) {
    saveLoopRange(pendingLoopStartTick, loopRange.endTick);
  } else {
    setNotice("Inicio A guardado", "Coloca el cursor después de A y fija el punto B para definir el rango.");
  }
});
elements.loopPointB.addEventListener("click", () => {
  const endTick = editCursorTick * 2;
  const startTick = pendingLoopStartTick ?? loopRange?.startTick;
  if (startTick === undefined || startTick === null || endTick <= startTick) {
    setNotice("Rango no válido", "Fija A antes de B y sitúa el punto B después de A.");
    return;
  }
  saveLoopRange(startTick, endTick);
});
elements.loopRangeClear.addEventListener("click", () => saveLoopRange(null, null));

async function saveLoopRange(startTick, endTick) {
  try {
    const snapshot = await platform.setLoopRange(startTick, endTick);
    renderSnapshot(snapshot);
    pendingLoopStartTick = null;
    setNotice("Rango de repetición guardado", loopRange
      ? `A: ${formatBarBeat(loopRange.startTick)} · B: ${formatBarBeat(loopRange.endTick)}. El loop de audio/MIDI aún no está conectado.`
      : "Se quitó el rango guardado del proyecto.");
  } catch (error) {
    setNotice("No se pudo guardar el rango", String(error));
  }
}
elements.importBar.addEventListener("change", () => {
  const bar = Math.max(1, Number(elements.importBar.value) || 1);
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  editCursorTick = Math.round((bar - 1) * beatsPerBar * 480);
  renderEditCursor();
});
elements.showArrangement.addEventListener("click", () => selectSurface("arrangement"));
elements.showSession.addEventListener("click", () => selectSurface("session"));
elements.showMixer.addEventListener("click", () => selectSurface("mixer"));

elements.newProject.addEventListener("click", async () => {
  await whileBusy([elements.newProject], async () => { try {
    const snapshot = await platform.newProject();
    renderSnapshot(snapshot);
    setNotice("Proyecto nuevo", "Sesión vacía lista. Abre un proyecto con clips MIDI para escuchar su reproducción.");
  } catch (error) {
    setNotice("No se pudo crear el proyecto", String(error));
  } });
});

elements.demoProject.addEventListener("click", async () => {
  await whileBusy([elements.demoProject, elements.browserDemo], async () => { try {
    const snapshot = await platform.demoMidiProject();
    renderSnapshot(snapshot);
    setNotice("Demo MIDI lista", "Siete notas están preparadas en la pista. Pulsa Play para oírlas por la salida configurada.");
  } catch (error) {
    setNotice("No se pudo preparar la demo MIDI", String(error));
  } });
});

elements.browserDemo.addEventListener("click", () => elements.demoProject.click());
elements.browserOpen.addEventListener("click", () => elements.open.click());

elements.save.addEventListener("click", () => runCommand("Proyecto guardado", () => platform.saveProject()));
elements.saveAs.addEventListener("click", () => runCommand("Copia del proyecto guardada", () => platform.saveProjectAs()));
elements.play.addEventListener("click", async () => {
  stopPreview();
  await whileBusy([elements.play], async () => { try {
    transportLoopErrorReported = false;
    const startAtCursor = projectTransportState === "stopped";
    const snapshot = await platform.setTransport("play", startAtCursor ? editCursorTick * 2 : null);
    renderSnapshot(snapshot);
    renderTransportPosition(await platform.transportPosition());
    const hasContent = snapshot.midiClipCount > 0 || snapshot.audioClipCount > 0;
    setNotice(hasContent ? "Transporte en Play" : "Sesión vacía", hasContent
      ? startAtCursor
        ? `Reproducción desde el cursor de Arreglo (${elements.transportPosition.textContent}).`
        : `Transporte reanudado en ${elements.transportPosition.textContent}.`
      : "No hay regiones MIDI ni de audio en este proyecto.");
  } catch (error) {
    setNotice("No se pudo iniciar la reproducción", String(error));
  } });
});
elements.pause.addEventListener("click", () => runCommand("Transporte pausado", () => platform.setTransport("pause")));
elements.stop.addEventListener("click", () => runCommand("Transporte detenido", () => platform.setTransport("stop")));
elements.panic.addEventListener("click", () => runCommand("Notas MIDI apagadas", () => platform.setTransport("panic")));
elements.metronome.addEventListener("click", async () => {
  const enabled = !metronomeEnabled;
  try {
    const snapshot = await platform.setTransport(enabled ? "metronome-on" : "metronome-off");
    metronomeEnabled = enabled;
    renderSnapshot(snapshot);
    elements.metronome.setAttribute("aria-pressed", String(enabled));
    elements.metronome.title = enabled ? "Desactivar metrónomo" : "Activar metrónomo";
    elements.metronome.classList.toggle("is-selected", enabled);
    setNotice(enabled ? "Metrónomo activado" : "Metrónomo desactivado", "El clic sigue el tempo y la métrica del proyecto.");
  } catch (error) {
    setNotice("No se pudo cambiar el metrónomo", String(error));
  }
});
elements.undo.addEventListener("click", () => runCommand("Undo aplicado", () => platform.historyAction("undo")));
elements.redo.addEventListener("click", () => runCommand("Redo aplicado", () => platform.historyAction("redo")));

setInterval(async () => {
  if (projectTransportState !== "playing" || transportPositionPollPending) return;
  transportPositionPollPending = true;
  try {
    renderTransportPosition(await platform.transportPosition());
  } catch (error) {
    if (!transportLoopErrorReported) {
      transportLoopErrorReported = true;
      setNotice("Falló la repetición A/B", String(error));
    }
  } finally {
    transportPositionPollPending = false;
  }
}, 50);

elements.audioProfile.addEventListener("change", () => {
  if (!audioSettings) return;
  audioSettings.activeProfile = elements.audioProfile.value;
  renderAudioProfile();
});

elements.saveAudioSettings.addEventListener("click", async () => {
  if (!audioSettings) return;
  const profile = selectedAudioProfile();
  profile.devicePeriodFrames = Number(elements.devicePeriod.value);
  profile.playbackSafetyFrames = Number(elements.playbackSafety.value);
  try {
    const view = await platform.saveAudioSettings(audioSettings);
    audioSettings = view.settings;
    elements.audioProfile.value = audioSettings.activeProfile;
    renderAudioProfile(view);
    setNotice("Preferencias de audio guardadas", "El perfil se usará al iniciar el siguiente stream. El cambio no modifica un stream que ya esté ejecutándose.");
  } catch (error) {
    setNotice("No se pudieron guardar los buffers", String(error));
  }
});

// Este shell inicial sólo resume datos compactos; jamás solicita PCM o buffers
// GPU al core a través del bridge.
setProjectEnabled(false);
elements.undo.disabled = true;
elements.redo.disabled = true;
renderTimelineRuler(4);
loadAudioSettings();
