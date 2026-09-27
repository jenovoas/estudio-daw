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
  addBusTrack: document.querySelector("#add-bus-track"),
  save: document.querySelector("#save-project"),
  saveAs: document.querySelector("#save-project-as"),
  zoomIn: document.querySelector("#zoom-in"),
  zoomOut: document.querySelector("#zoom-out"),
  zoomReset: document.querySelector("#zoom-reset"),
  zoomLevel: document.querySelector("#zoom-level"),
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
let selectedTrackIds = new Set();
let trackGroupDraft = "";
let pendingAudioPath = null;
let editCursorTick = 0;
let editCursorProjectId = null;
const waveformCache = new Map();
let previewContext = null;
let currentPreview = null;
const UI_ZOOM_STORAGE_KEY = "estudio-daw.ui-zoom.v1";
const UI_ZOOM_MIN = 0.8;
const UI_ZOOM_MAX = 1.5;
const UI_ZOOM_STEP = 0.1;
let uiZoom = 1;

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

function updateUiZoomControls() {
  const percent = Math.round(uiZoom * 100);
  elements.zoomLevel.textContent = `${percent}%`;
  elements.zoomIn.disabled = uiZoom >= UI_ZOOM_MAX;
  elements.zoomOut.disabled = uiZoom <= UI_ZOOM_MIN;
  elements.zoomReset.title = `Restablecer interfaz (Ctrl+0); actual ${percent}%`;
}

async function setUiZoom(scale, persist = true) {
  const clamped = Math.min(UI_ZOOM_MAX, Math.max(UI_ZOOM_MIN, scale));
  const next = Math.round(clamped * 100) / 100;
  try {
    await platform.setUiZoom(next);
  } catch (error) {
    setNotice("No se pudo cambiar el tamaño", String(error));
    return;
  }
  uiZoom = next;
  if (persist) {
    try {
      localStorage.setItem(UI_ZOOM_STORAGE_KEY, String(next));
    } catch {
      // El zoom sigue disponible en la sesión aunque el WebView no guarde preferencias.
    }
  }
  updateUiZoomControls();
}

function initializeUiZoom() {
  let stored = 1;
  try {
    stored = Number(localStorage.getItem(UI_ZOOM_STORAGE_KEY));
  } catch {
    stored = 1;
  }
  const initial = Number.isFinite(stored) && stored > 0 ? stored : 1;
  uiZoom = 1;
  updateUiZoomControls();
  if (initial !== 1) void setUiZoom(initial, false);
}

function handleUiZoomShortcut(event) {
  if (!event.ctrlKey || event.altKey || event.metaKey) return;
  if (event.target instanceof Element && event.target.closest("input, textarea, select, [contenteditable='true']")) return;
  if (["+", "="].includes(event.key) || ["NumpadAdd"].includes(event.code)) {
    event.preventDefault();
    void setUiZoom(uiZoom + UI_ZOOM_STEP);
  } else if (["-", "_"].includes(event.key) || event.code === "NumpadSubtract") {
    event.preventDefault();
    void setUiZoom(uiZoom - UI_ZOOM_STEP);
  } else if (event.key === "0" || event.code === "Numpad0") {
    event.preventDefault();
    void setUiZoom(1);
  }
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
  for (const button of [elements.save, elements.saveAs, elements.play, elements.pause, elements.stop, elements.addMidiTrack, elements.addAudioTrack, elements.addBusTrack]) {
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

function trackSignalFlow(track, tracks) {
  if (track.virtualMaster || track.role === "master") {
    return "Suma MIDI/audio/metrónomo → ganancia/silencio Master → medidor Master → salida del plan";
  }
  const output = tracks.find((candidate) => candidate.id === track.outputTrackId);
  const destination = output?.name ?? "Master";
  if (track.role === "bus") {
    return `Suma de pistas enrutadas → ganancia/pan de bus → medidor → ${destination}`;
  }
  if (track.kind === "audio") {
    return `Regiones de audio → ganancia/desvanecimientos de región → ganancia/pan → medidor → ${destination}`;
  }
  return `Eventos MIDI → instrumento → ganancia/pan → medidor → ${destination}`;
}

function createTrackOutputControl(track, tracks) {
  if (track.virtualMaster || track.role === "master") return null;
  const targets = tracks.filter((candidate) => ["audio", "bus", "master"].includes(candidate.role));
  const hasPersistedMaster = targets.some((candidate) => candidate.role === "master");
  if (!hasPersistedMaster) {
    targets.push({ id: "", name: "Master", role: "master", virtualMaster: true });
  }
  const field = document.createElement("label");
  field.className = "mixer-output-select";
  const caption = document.createElement("span");
  caption.textContent = "Salida";
  const select = document.createElement("select");
  select.setAttribute("aria-label", `Salida de ${track.name}`);
  for (const target of targets) {
    if (target.id === track.id) continue;
    const option = document.createElement("option");
    option.value = target.virtualMaster ? "" : target.id;
    option.textContent = target.role === "master" ? "Master" : target.name;
    select.append(option);
  }
  select.value = track.outputTrackId ?? targets.find((target) => target.role === "master")?.id ?? "";
  select.addEventListener("change", () => updateTrackOutput(track, select.value));
  field.append(caption, select);
  return field;
}

function createTrackMixerControls(track, compact = false) {
  if (track.virtualMaster) return null;
  const isMaster = track.role === "master";
  const controls = document.createElement("div");
  controls.className = compact ? "mixer-controls mixer-controls-compact" : "mixer-controls";
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
  if (!isMaster) toggle("S", "solo", track.solo);

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
  if (!isMaster) slider("Pan", "pan", track.pan, -1, 1, 0.05);
  return controls;
}

function createTrackMeter(track) {
  const meter = document.createElement("div");
  meter.className = "track-meter";
  meter.dataset.trackId = track.role === "master" ? "__master__" : track.id;
  meter.setAttribute("role", "meter");
  meter.setAttribute("aria-valuemin", "0");
  meter.setAttribute("aria-valuemax", "100");
  meter.setAttribute("aria-valuenow", "0");
  meter.setAttribute("aria-label", `Nivel de ${track.name}`);
  const fill = document.createElement("span");
  fill.className = "track-meter-fill";
  meter.append(fill);
  return meter;
}

function syncTrackSelectionUi() {
  for (const checkbox of document.querySelectorAll("[data-track-select]")) {
    checkbox.checked = selectedTrackIds.has(checkbox.dataset.trackSelect);
    checkbox.closest(".track-row, .session-track-card, .mixer-channel")
      ?.classList.toggle("is-track-selected", checkbox.checked);
  }
  const count = document.querySelector(".track-group-selection-count");
  if (count) count.textContent = `${selectedTrackIds.size} seleccionadas`;
  const canApply = selectedTrackIds.size > 0;
  for (const button of document.querySelectorAll("[data-track-group-action]")) {
    button.disabled = !canApply || (button.dataset.trackGroupAction === "assign" && !trackGroupDraft.trim());
  }
}

function createTrackSelectionControl(track) {
  if (track.virtualMaster) return null;
  const label = document.createElement("label");
  label.className = "track-select";
  label.title = `Seleccionar pista ${track.name}`;
  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.dataset.trackSelect = track.id;
  checkbox.checked = selectedTrackIds.has(track.id);
  checkbox.setAttribute("aria-label", `Seleccionar pista ${track.name}`);
  checkbox.addEventListener("change", () => {
    if (checkbox.checked) selectedTrackIds.add(track.id);
    else selectedTrackIds.delete(track.id);
    syncTrackSelectionUi();
  });
  label.append(checkbox);
  return label;
}

function createTrackRemovalButton(track) {
  if (track.virtualMaster || track.role === "master") return null;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "track-remove-button";
  button.textContent = "×";
  button.title = `Quitar ${track.name}; se puede deshacer`;
  button.setAttribute("aria-label", `Quitar pista ${track.name}`);
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    removeTrackFromProject(track, button);
  });
  return button;
}

async function removeTrackFromProject(track, button) {
  await whileBusy([button], async () => {
    try {
      const snapshot = await platform.removeTrack(track.id);
      selectedTrackIds.delete(track.id);
      renderSnapshot(snapshot);
      setNotice(
        "Pista quitada",
        "Se quitaron la pista y sus clips del proyecto. Puedes deshacerlo; los archivos de medios no se borraron.",
      );
    } catch (error) {
      setNotice("No se pudo quitar la pista", String(error));
    }
  });
}

function createTrackGroupToolbar() {
  const toolbar = document.createElement("div");
  toolbar.className = "track-group-toolbar";
  const input = document.createElement("input");
  input.type = "text";
  input.maxLength = 64;
  input.placeholder = "Nombre del grupo";
  input.value = trackGroupDraft;
  input.setAttribute("aria-label", "Nombre del grupo de pistas");
  input.addEventListener("input", () => {
    trackGroupDraft = input.value;
    syncTrackSelectionUi();
  });
  const assign = document.createElement("button");
  assign.type = "button";
  assign.textContent = "Agrupar selección";
  assign.dataset.trackGroupAction = "assign";
  assign.addEventListener("click", () => updateSelectedTrackGroup(trackGroupDraft.trim() || null));
  const clear = document.createElement("button");
  clear.type = "button";
  clear.textContent = "Quitar del grupo";
  clear.dataset.trackGroupAction = "clear";
  clear.addEventListener("click", () => updateSelectedTrackGroup(null));
  const count = document.createElement("span");
  count.className = "track-group-selection-count";
  toolbar.append(input, assign, clear, count);
  return toolbar;
}

async function updateSelectedTrackGroup(groupName) {
  if (selectedTrackIds.size === 0) return;
  await runCommand(
    groupName ? "Grupo de pistas actualizado" : "Pistas desagrupadas",
    () => platform.setTracksGroup([...selectedTrackIds], groupName),
  );
}

function updateTrackMeters(meters) {
  for (const meter of document.querySelectorAll(".track-meter")) {
    const reading = meters[meter.dataset.trackId];
    const peak = Math.max(0, Number(reading?.peak) || 0);
    const rms = Math.max(0, Number(reading?.rms) || 0);
    const level = Math.min(100, peak * 100);
    meter.style.setProperty("--meter-level", `${level}%`);
    meter.setAttribute("aria-valuenow", level.toFixed(0));
    meter.title = `Pico ${(20 * Math.log10(Math.max(peak, 1e-6))).toFixed(1)} dBFS · RMS ${(20 * Math.log10(Math.max(rms, 1e-6))).toFixed(1)} dBFS`;
  }
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
    type.textContent = track.role === "master" ? "MASTER" : track.role === "bus" ? "BUS" : track.kind.toUpperCase();
    const channels = document.createElement("small");
    channels.textContent = trackChannelDescription(track);
    if (track.groupName) channels.textContent += ` · Grupo: ${track.groupName}`;
    const emptySlot = document.createElement("div");
    emptySlot.className = "session-empty-slot";
    emptySlot.textContent = "Sin escena";
    const headingRow = document.createElement("div");
    headingRow.className = "session-track-heading";
    headingRow.append(name);
    const removeButton = createTrackRemovalButton(track);
    if (removeButton) headingRow.append(removeButton);
    column.append(createTrackSelectionControl(track), headingRow, type, channels, emptySlot);
    const meter = createTrackMeter(track);
    if (meter) column.append(meter);
    const controls = createTrackMixerControls(track, true);
    if (controls) column.append(controls);
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
  }
  const channels = document.createElement("div");
  channels.className = "mixer-channel-list";
  const mixerTracks = tracks.some((track) => track.role === "master")
    ? tracks
    : [...tracks, {
      id: "__master__",
      name: "Master",
      kind: "audio",
      role: "master",
      color: "#d5a36f",
      outputChannels: 2,
      active: true,
      mute: false,
      solo: false,
      gainDb: 0,
      pan: 0,
      virtualMaster: true,
    }];
  for (const track of mixerTracks) {
    const channel = document.createElement("article");
    channel.className = "mixer-channel";
    channel.style.setProperty("--track-color", track.color);
    const title = document.createElement("strong");
    title.textContent = track.name;
    const role = document.createElement("span");
    role.className = "mixer-role";
    role.textContent = track.role === "master"
      ? "MASTER"
      : track.role === "bus" ? "BUS" : track.kind.toUpperCase();
    const routing = document.createElement("small");
    routing.className = "mixer-flow";
    routing.textContent = trackSignalFlow(track, tracks);
    const mix = document.createElement("span");
    mix.className = "mixer-values";
    mix.textContent = track.virtualMaster
      ? "Salida estéreo combinada"
      : track.role === "master"
        ? `${Number(track.gainDb).toFixed(1)} dB${track.mute ? " · Silencio" : ""}${track.active ? "" : " · Inactiva"}`
        : `${Number(track.gainDb).toFixed(1)} dB · Pan ${Number(track.pan).toFixed(2)}${track.mute ? " · Silencio" : ""}${track.solo ? " · Solo" : ""}${track.active ? "" : " · Inactiva"}`;
    if (track.groupName) role.textContent += ` · ${track.groupName}`;
    const selection = createTrackSelectionControl(track);
    const removeButton = createTrackRemovalButton(track);
    const outputControl = createTrackOutputControl(track, tracks);
    const channelHeading = document.createElement("div");
    channelHeading.className = "mixer-channel-heading";
    if (selection) channelHeading.append(selection);
    channelHeading.append(title);
    if (removeButton) channelHeading.append(removeButton);
    channel.append(channelHeading, role, routing, mix);
    if (outputControl) channel.append(outputControl);
    const meter = createTrackMeter(track);
    if (meter) channel.append(meter);
    const controls = createTrackMixerControls(track);
    if (controls) channel.append(controls);
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

function updateTrackOutput(track, outputTrackId) {
  return runCommand("Salida actualizada", () => platform.setTrackOutput(
    track.id,
    outputTrackId || null,
  ));
}

function renderSnapshot(snapshot) {
  setProjectEnabled(true);
  if (snapshot.projectId !== editCursorProjectId) {
    selectedTrackIds = new Set();
    trackGroupDraft = "";
    editCursorTick = 0;
    transportPositionTick = 0;
    editCursorProjectId = snapshot.projectId;
  }
  const validTrackIds = new Set(snapshot.tracks.map((track) => track.id));
  selectedTrackIds = new Set([...selectedTrackIds].filter((trackId) => validTrackIds.has(trackId)));
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
  if (!document.querySelector(".track-group-toolbar")) {
    document.querySelector(".editor-heading").append(createTrackGroupToolbar());
  }
  const groupInput = document.querySelector(".track-group-toolbar input");
  if (groupInput && groupInput.value !== trackGroupDraft) groupInput.value = trackGroupDraft;
  syncTrackSelectionUi();
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
    icon.textContent = track.role === "master" ? "M" : track.role === "bus" ? "B" : track.kind === "audio" ? "◖" : "♫";
    const label = document.createElement("span");
    label.textContent = track.name;
    const details = document.createElement("span");
    details.className = "track-meta";
    const mixState = [track.mute ? "MUTE" : null, track.solo ? "SOLO" : null, !track.active ? "OFF" : null].filter(Boolean).join(" · ");
    details.textContent = `${track.role === "master" ? "MASTER" : track.role === "bus" ? "BUS" : track.kind === "audio" ? "AUDIO" : "MIDI"}${track.kind === "midi" ? ` · ${track.noteCount} notas` : track.role === "audio" ? ` · ${track.outputChannels} ch` : ""}${track.groupName ? ` · GRUPO ${track.groupName}` : ""}${mixState ? ` · ${mixState}` : ""}`;
    const name = document.createElement("div");
    name.className = "track-name";
    name.append(createTrackSelectionControl(track), icon, label);
    const headingRow = document.createElement("div");
    headingRow.className = "track-row-heading";
    headingRow.append(name);
    const removeButton = createTrackRemovalButton(track);
    if (removeButton) headingRow.append(removeButton);
    row.append(headingRow, details);
    const meter = createTrackMeter(track);
    if (meter) row.append(meter);
    const mixerControls = createTrackMixerControls(track, true);
    if (mixerControls) row.append(mixerControls);
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
  syncTrackSelectionUi();
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
    const label = kind === "audio" ? "Pista de audio" : kind === "bus" ? "Bus" : "Pista MIDI";
    setNotice(`${label} creada`, "La pista quedó en el proyecto y su creación puede deshacerse desde el historial.");
  } catch (error) {
    setNotice("No se pudo crear la pista", String(error));
  } });
}

elements.addMidiTrack.addEventListener("click", () => addTrack("midi", elements.addMidiTrack));
elements.addAudioTrack.addEventListener("click", () => addTrack("audio", elements.addAudioTrack));
elements.addBusTrack.addEventListener("click", () => addTrack("bus", elements.addBusTrack));
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
  if (!["playing", "paused"].includes(projectTransportState) || transportPositionPollPending) return;
  transportPositionPollPending = true;
  try {
    if (projectTransportState === "playing") {
      renderTransportPosition(await platform.transportPosition());
    }
    updateTrackMeters(await platform.trackMeters());
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

elements.zoomIn.addEventListener("click", () => void setUiZoom(uiZoom + UI_ZOOM_STEP));
elements.zoomOut.addEventListener("click", () => void setUiZoom(uiZoom - UI_ZOOM_STEP));
elements.zoomReset.addEventListener("click", () => void setUiZoom(1));
document.addEventListener("keydown", handleUiZoomShortcut);

// Este shell inicial sólo resume datos compactos; jamás solicita PCM o buffers
// GPU al core a través del bridge.
setProjectEnabled(false);
elements.undo.disabled = true;
elements.redo.disabled = true;
renderTimelineRuler(4);
initializeUiZoom();
loadAudioSettings();
