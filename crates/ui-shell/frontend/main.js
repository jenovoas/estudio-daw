const platform = window.estudioPlatform;

const elements = {
  open: document.querySelector("#open-project"),
  browserOpen: document.querySelector("#browser-open"),
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
  heading.innerHTML = "<strong>MEZCLADOR</strong><span>Ruteo de proyecto · controles de señal aún no conectados</span>";
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
    channels.append(channel);
  }
  elements.mixerView.append(channels);
}

function renderSnapshot(snapshot) {
  setProjectEnabled(true);
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
    if (clips.length === 0) {
      const empty = document.createElement("span");
      empty.className = "lane-empty-label";
      empty.textContent = "Sin clips";
      lane.append(empty);
    }
    elements.lanes.append(lane);
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
  elements.ruler.dataset.beatsPerBar = String(beatsPerBar);
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
  await whileBusy([elements.play], async () => { try {
    const snapshot = await platform.setTransport("play");
    renderSnapshot(snapshot);
    setNotice(snapshot.midiClipCount === 0
      ? "Sesión vacía"
      : "Transporte en Play", snapshot.midiClipCount === 0
      ? "No hay clips MIDI en esta sesión; abre un proyecto con clips para escuchar instrumentos."
      : "Reproduciendo clips MIDI con los instrumentos asignados a sus pistas.");
  } catch (error) {
    setNotice("No se pudo iniciar la reproducción", String(error));
  } });
});
elements.pause.addEventListener("click", () => runCommand("Transporte pausado", () => platform.setTransport("pause")));
elements.stop.addEventListener("click", () => runCommand("Transporte detenido", () => platform.setTransport("stop")));
elements.undo.addEventListener("click", () => runCommand("Undo aplicado", () => platform.historyAction("undo")));
elements.redo.addEventListener("click", () => runCommand("Redo aplicado", () => platform.historyAction("redo")));

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
