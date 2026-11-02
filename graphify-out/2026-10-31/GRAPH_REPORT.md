# Graph Report - estudio-daw  (2026-10-31)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 3989 nodes · 9214 edges · 149 communities (124 shown, 25 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 248 edges (avg confidence: 0.72)
- Token cost: 59,393 input · 1,808 output

## Graph Freshness
- Built from commit: `eff2342d`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Modelo de Proyecto
- Adaptador de Medios
- Implementación del Plugin VST3
- Interfaz del Plugin VST3
- Intercambio de Datos
- Estado de Escritorio y Audio
- Plan de Renderizado de Audio
- Controladores MIDI
- Bloques de Audio
- Comandos de la CLI
- Comandos del Host VST3
- Formato Binario VST3
- Sonda y Descubrimiento VST3
- Implementación del Plugin VST3
- Reproducción del Proyecto
- Buffers y Medición de Audio
- Bus de Comandos del Proyecto
- Ventana del Editor VST3
- Integración COM del Host VST3
- Estado y Acciones de Escritorio
- Edición de Parámetros VST3
- Canales MIDI
- Aislamiento del Plugin VST3
- Configuración de la Aplicación
- Comandos del Proyecto
- Aislamiento de Procesos VST3
- Eventos MIDI
- Gestión de Ventanas del Plugin
- Backend de Audio CPAL
- Sesión y Reloj de Transporte
- Lista de Eventos VST3
- Interfaz Interna del Plugin
- Respuestas del Host VST3
- Reproducción MIDI
- Validación del Modelo de Pistas
- Motor de Síntesis y Mezcla
- Colas de Comandos en Tiempo Real
- Puente de Audio de Sesión
- Interfaz de Arreglo
- Comandos e Historial del Proyecto
- Procesamiento DSP y Ecualización
- Lectura de Metadatos del Plugin
- Seguridad y Autorización de Plugins
- Buffers de Audio por Bus
- Worker de FluidSynth
- Streams de Estado VST3
- Enlace con FluidSynth
- Grabación MIDI
- Motor FluidSynth
- Host de Audio y Runtime
- Transporte y Coordinación MIDI
- Pruebas de Asignación en Tiempo Real
- Descubrimiento de Plugins VST3
- Vista del Editor VST3
- Aplicación e Historial del Proyecto
- Interfaz de Arreglo y Clips
- Integración de Ventanas macOS
- Gestión del Editor Embebido
- CLI de Audio y PipeWire
- Control del Stream de Audio
- Carga de Bundles macOS
- Host VST3 y Entrada MIDI
- Nodos de Audio DSP
- Controles MIDI de Transporte
- Diagnóstico de Dispositivos MIDI
- Vista de Plugin de Prueba
- Control del Plugin de Audio
- Carga de Plugins macOS
- Ciclo de Vida del Host VST3
- Comandos de Clips de Audio
- Colas de Automatización VST3
- Curvas de Automatización
- Entrada MIDI ALSA
- Atributos del Host VST3
- Conexión Componente-Controlador
- Carga de Módulos VST3
- Estado y Audio del Plugin VST3
- Controles de Cabecera de Pista
- Notificaciones del Host VST3
- Detección de Arquitectura de Plugin
- Mezcla de Audio de Sesión
- Proxy de Conexión VST3
- Banderas de Reinicio VST3
- Configuración de Buses del Plugin
- Menús Contextuales VST3
- Carga y Reproducción de Plugins
- Historial y Eventos del Proyecto
- Colas de Eventos MIDI de Salida
- Estado y Presets VST3
- Runtime de Cómputo y GPU
- Procesamiento de Buffers por Bloques
- Utilidades de Identificadores VST3
- Proceso Host de Plugins
- Errores del Host VST3
- Conversión de Parámetros VST3
- Interfaz del Navegador de Plugins
- Configuración de la Aplicación Tauri
- Crates del Dominio y Runtime
- Decodificación de Audio de Sesión
- Eventos MIDI del Worker
- Linux Plugin Loading
- Plugin Helper Recovery
- IPC Exchange Errors
- COM Host Messaging
- Compute Backend Selection
- MIDI Clip Editing
- Plugin Compatibility Validation
- Denormal Number Handling
- Audio Codec Serialization
- Live Transport State
- Live ALSA MIDI
- Clip Launch Quantization
- Workstation UI Actions
- Audio Device Backend
- Audio Level Metering
- Plugin Bus Arrangements
- Plugin Event Data
- Recorded MIDI Messages
- Plugin Snapshot Discovery
- Plugin Parameter Conversion
- Audio Compute Benchmarks
- Compute Workload Types
- Session Clip Launching
- Plugin Bus Configuration
- Plugin Editor Windows
- Plugin Isolation Errors
- Bounded IPC Line Reading
- FFT Implementation
- Windows Permissions Configuration
- Plugin Probe Outcomes
- IPC Error Recovery
- Automation Modes
- Plugin Factory Flags
- Plugin Process Modes
- Audio Preview Controls
- Plugin Snapshot Tests
- Plugin Parameter Updates
- Plugin Runtime Status
- Protected Plugin Calls
- Plugin State Encoding
- Musical Note Naming
- Audio Channel Pointers
- Compute Runtime

## God Nodes (most connected - your core abstractions)
1. `Plugin` - 158 edges
2. `PluginImpl` - 138 edges
3. `Other` - 91 edges
4. `Project` - 88 edges
5. `HostCommand` - 83 edges
6. `IsolatedPluginImpl` - 78 edges
7. `PluginInternal` - 78 edges
8. `ProjectCommand` - 59 edges
9. `MidiEvent` - 59 edges
10. `DesktopState` - 57 edges

## Surprising Connections (you probably didn't know these)
- `render_plan_handoff_at_audio_boundary_does_not_allocate()` --calls--> `render_plan_exchange()`  [INFERRED]
  crates/audio-engine/tests/realtime_no_alloc.rs → crates/audio-engine/src/lib.rs
- `HostCommand` --references--> `AudioBusBuffer`  [INFERRED]
  crates/vst3-host/src/process_isolation.rs → crates/vst3-host/src/audio.rs
- `HostCommand` --references--> `BusDirection`  [INFERRED]
  crates/vst3-host/src/process_isolation.rs → crates/vst3-host/src/audio.rs
- `HostCommand` --references--> `MediaType`  [INFERRED]
  crates/vst3-host/src/process_isolation.rs → crates/vst3-host/src/audio.rs
- `HostCommand` --references--> `SpeakerArrangement`  [INFERRED]
  crates/vst3-host/src/process_isolation.rs → crates/vst3-host/src/audio.rs

## Import Cycles
- 1-file cycle: `crates/vst3-host/src/process_isolation.rs -> crates/vst3-host/src/process_isolation.rs`
- 1-file cycle: `crates/vst3-host/src/plugin.rs -> crates/vst3-host/src/plugin.rs`
- 1-file cycle: `crates/vst3-host/src/window.rs -> crates/vst3-host/src/window.rs`
- 1-file cycle: `crates/vst3-host/src/embed.rs -> crates/vst3-host/src/embed.rs`
- 1-file cycle: `crates/vst3-host/src/error.rs -> crates/vst3-host/src/error.rs`
- 1-file cycle: `crates/vst3-host/src/internal/denormal.rs -> crates/vst3-host/src/internal/denormal.rs`
- 2-file cycle: `crates/synth/src/fluidsynth_worker.rs -> crates/synth/src/lib.rs -> crates/synth/src/fluidsynth_worker.rs`

## Communities (149 total, 25 thin omitted)

### Community 0 - "Modelo de Proyecto"
Cohesion: 0.04
Nodes (66): attach_midi_take(), attaches_midi_take_to_first_midi_track(), AttachTakeError, NoMidiTrack, AudioPlaylist, clip_slots_reference_existing_clip_without_copying_content(), current_schema_repairs_missing_role_and_channel_defaults_from_track_kind(), DawArrangement (+58 more)

### Community 1 - "Adaptador de Medios"
Cohesion: 0.06
Nodes (60): proxy_audio_command(), audio_preview_ogg(), audio_waveform(), AudioFileMetadata, AudioPcmDecoder, AudioPcmDecoderControl, AudioProxyProfile, building_marker() (+52 more)

### Community 2 - "Implementación del Plugin VST3"
Cohesion: 0.05
Nodes (9): AppliedSetup, current_system_time_nanos(), midi_mapping_bus_count(), PluginImpl, process_context_needs(), ProcessingSampleSize, F32, F64 (+1 more)

### Community 3 - "Interfaz del Plugin VST3"
Cohesion: 0.05
Nodes (3): Plugin, Other, validate_plugin_event()

### Community 4 - "Intercambio de Datos"
Cohesion: 0.05
Nodes (16): AlignedBlock, BLOCK_AVAILABLE, BLOCK_LOCKED, BLOCK_PENDING, DataExchangeState, exchange_queue_exhausts_and_recycles_without_allocating(), ExchangeQueue, MAX_BLOCK_SIZE (+8 more)

### Community 5 - "Estado de Escritorio y Audio"
Cohesion: 0.05
Nodes (53): TrackInputRoute, SessionLaunchView, apply_authorized_external_code(), audio_input_devices(), audio_output_devices(), audio_preview(), audio_preview_file(), audio_return_devices() (+45 more)

### Community 6 - "Plan de Renderizado de Audio"
Cohesion: 0.11
Nodes (39): render_plan_exchange(), RenderPlan, RenderPlanProcessor, audio_params(), output_buffer_bytes(), pause_outputs_silence_without_advancing_render_plan(), PipeWireDuplexReport, PipeWireError (+31 more)

### Community 7 - "Controladores MIDI"
Cohesion: 0.03
Nodes (50): ALL_NOTES_OFF, ALL_SOUNDS_OFF, BALANCE, BANK_SELECT_MSB, BREATH, CELESTE_DEPTH, CHORUS_DEPTH, DATA_DECREMENT (+42 more)

### Community 8 - "Bloques de Audio"
Cohesion: 0.06
Nodes (27): AudioBlock, AudioBlockError, InvalidChannelCount, InvalidFrameCount, DropCounter, empty_active_slot_fails_closed_with_silence_without_panicking(), equalizer_bypass_preserves_signal_while_enabled_band_changes_it(), exchange_rejects_another_plan_until_retired_slot_is_reclaimed() (+19 more)

### Community 9 - "Comandos de la CLI"
Cohesion: 0.05
Nodes (27): add_audio_clip_command(), apply_project_command_file(), attach_media_command(), attach_take_command(), defaults_to_sine_and_requires_soundfont_for_nondefault_preset(), devices_command(), export_command(), import_command() (+19 more)

### Community 10 - "Comandos del Host VST3"
Cohesion: 0.03
Nodes (59): HostCommand, AudioBusLayout, BeginHostEdit, BusArrangements, CloseGui, CreateGui, DismissContextMenu, EndHostEdit (+51 more)

### Community 11 - "Formato Binario VST3"
Cohesion: 0.05
Nodes (44): MAX_EVENT_PAYLOAD_BYTES, MAX_EVENT_TEXT_UNITS, build(), build_parse_round_trip(), CLASS_ID_LEN, COMPONENT_CHUNK, CONTROLLER_CHUNK, ENTRY_SIZE (+36 more)

### Community 12 - "Sonda y Descubrimiento VST3"
Cohesion: 0.05
Nodes (18): main(), builder_rejects_time_signatures_the_runtime_rejects(), builder_threads_tempo_and_time_signature_into_config(), clamp_output_channels(), DiscoveryProgress, Completed, Error, Found (+10 more)

### Community 13 - "Implementación del Plugin VST3"
Cohesion: 0.06
Nodes (37): a_declined_scale_factor_is_not_an_error(), a_nonsensical_scale_factor_is_rejected_without_reaching_the_view(), advance_moves_playhead_and_musical_time(), advance_process_context(), an_accepted_scale_factor_reports_support(), blank_event(), BusActivationState, converts_legacy_cc_and_pitchbend() (+29 more)

### Community 14 - "Reproducción del Proyecto"
Cohesion: 0.07
Nodes (42): MidiClip, TimeSignature, Track, TrackMixerState, Transport, build_project_playback(), build_project_playback_with_end(), clip_duration_session_ticks() (+34 more)

### Community 15 - "Buffers y Medición de Audio"
Cohesion: 0.06
Nodes (28): InputSource, noise_is_bounded_and_varied(), peak_meter_hold_falls_after_window(), peak_meter_level_falls_but_hold_latches(), peak_meter_reaches_silence_floor(), peak_meter_rises_instantly_and_holds(), PeakMeter, .SILENCE (+20 more)

### Community 16 - "Bus de Comandos del Proyecto"
Cohesion: 0.05
Nodes (30): COMMAND_SCHEMA_VERSION, CommandBusError, Closed, Full, CommandDiagnostic, CommandEnvelope, CommandError, EmptyId (+22 more)

### Community 17 - "Ventana del Editor VST3"
Cohesion: 0.08
Nodes (20): close_editor(), close_editor_window(), connect_xwayland(), err(), gui_request(), GuiChannel, GuiRequest, handle() (+12 more)

### Community 18 - "Integración COM del Host VST3"
Cohesion: 0.07
Nodes (29): com_write_past_the_cap_reports_an_error_code(), create_host_application(), create_host_plug_frame(), DROP_LOG_INTERVAL, HostPlugFrame, huge_seek_then_write_is_refused_instead_of_allocating(), interface_support_reports_plugin_side_interfaces_only(), make_frame() (+21 more)

### Community 19 - "Estado y Acciones de Escritorio"
Cohesion: 0.16
Nodes (48): load_audio_runtime_settings(), add_midi_note(), add_scene(), add_track(), audio_runtime_settings(), audio_waveform(), create_midi_clip(), demo_midi_project() (+40 more)

### Community 20 - "Edición de Parámetros VST3"
Cohesion: 0.06
Nodes (15): captures_begin_perform_end_in_order_and_drains(), ComponentHandler, context_menu_item(), create_instance_requires_matching_class_and_interface_ids(), editor_feedback_is_capped_when_the_host_never_polls(), handler2_requests_are_ordered_and_report_backpressure(), handler3_context_menu_preserves_items_and_executes_plugin_target(), handler3_rejects_invalid_views_and_releases_dismissed_targets() (+7 more)

### Community 21 - "Canales MIDI"
Cohesion: 0.05
Nodes (30): MidiChannel, Ch1, Ch10, Ch11, Ch12, Ch13, Ch14, Ch15 (+22 more)

### Community 23 - "Configuración de la Aplicación"
Cohesion: 0.08
Nodes (30): Io, atomic_write(), audio_runtime_settings_default_and_round_trip_are_versioned(), audio_runtime_settings_path(), audio_runtime_settings_reject_unknown_versions_and_invalid_periods(), audio_runtime_view_keeps_requested_period_distinct_from_unknown_effective_period(), AUDIO_SETTINGS_SCHEMA, AudioProfile (+22 more)

### Community 24 - "Comandos del Proyecto"
Cohesion: 0.04
Nodes (47): ProjectCommand, AddAudioClip, AddMidiNote, AddScene, AddTrack, AttachMediaSource, AttachMidiTake, CreateMidiClip (+39 more)

### Community 25 - "Aislamiento de Procesos VST3"
Cohesion: 0.04
Nodes (14): ALPHABET, DEFAULT_RESPONSE_TIMEOUT, DEFAULT_SLOW_COMMAND_TIMEOUT, IsolationResult, LOAD_CRASH_RETRIES, LOAD_CRASH_RETRY_BACKOFF, MAX_QUEUED_RESPONSES, MAX_RESPONSE_LINE_BYTES (+6 more)

### Community 26 - "Eventos MIDI"
Cohesion: 0.09
Nodes (21): AudioBuffers, MidiEvent, ChannelAftertouch, ControlChange, NoteOff, NoteOn, PitchBend, PolyAftertouch (+13 more)

### Community 27 - "Gestión de Ventanas del Plugin"
Cohesion: 0.08
Nodes (8): close_requests(), dpi_changes(), dpi_from_wparam(), dpi_scale_factor(), plugin_window_proc(), PluginWindow, PluginWindowBuilder, XcbWindowState

### Community 28 - "Backend de Audio CPAL"
Cohesion: 0.07
Nodes (7): clamp_block_to_buffer_size(), CpalBackend, CpalStream, range(), resolve_buffer_size(), resolve_input_buffer_size(), resolve_output_buffer_size()

### Community 29 - "Sesión y Reloj de Transporte"
Cohesion: 0.08
Nodes (24): bounded_bus_applies_commands_in_order(), clock_is_deterministic_across_callback_boundaries(), clock_stays_stable_across_variable_blocks_at_fractional_tempo(), CommandBus, CommandBusError, Closed, Full, full_bus_never_blocks_the_producer() (+16 more)

### Community 30 - "Lista de Eventos VST3"
Cohesion: 0.08
Nodes (16): Event, create_event_list(), HostEventList, malformed_or_oversized_pointer_payloads_are_rejected(), plugin_event_to_raw(), queued_events_are_capped_so_a_stopped_plugin_cannot_grow_them(), raw_event_to_plugin_event(), sysex_is_deep_copied_before_plugin_memory_expires() (+8 more)

### Community 32 - "Respuestas del Host VST3"
Cohesion: 0.05
Nodes (32): NoteExpressionInfo, PluginUnit, ProgramPitchName, HostResponse, AudioBusLayout, AudioOutput, BusArrangements, BusAudioOutput (+24 more)

### Community 33 - "Reproducción MIDI"
Cohesion: 0.09
Nodes (21): midi_play_command(), project_play_command(), DEFAULT_PPQ, LiveMidiOutputWorker, play_midi_take(), play_midi_take_interactive(), play_midi_take_internal(), play_midi_take_live() (+13 more)

### Community 34 - "Validación del Modelo de Pistas"
Cohesion: 0.06
Nodes (35): all_track_roles_have_valid_serializable_construction_contracts(), default_track_color(), project_save_reopen_preserves_standalone_executable_and_routes(), project_track_validation_rejects_duplicate_ids_and_multiple_masters(), track_eq_bands_roundtrip_and_legacy_default(), TrackKind, Audio, TrackRole (+27 more)

### Community 35 - "Motor de Síntesis y Mezcla"
Cohesion: 0.10
Nodes (27): all_notes_off_releases_only_the_selected_channel(), ATTACK_SECONDS, ConstantNode, EventRing, explicit_instrument_mixer_sums_parallel_sources(), InstrumentMixerNode, MIDI_EVENT_CAPACITY, midi_event_queue() (+19 more)

### Community 36 - "Colas de Comandos en Tiempo Real"
Cohesion: 0.09
Nodes (17): control_queue_reports_full_without_blocking(), disconnected_or_full_handoff_leaks_instead_of_dropping_the_value(), drain_commands(), drain_commands_stops_after_one_ring_even_while_the_producer_refills(), drop_probe(), dropping_teardown_receiver_off_owner_leaks_queued_value(), DropProbe, invalid_transport_values_are_rejected_not_queued() (+9 more)

### Community 37 - "Puente de Audio de Sesión"
Cohesion: 0.11
Nodes (22): AudioSideChannels, bridge_capacity_is_a_whole_number_of_frames(), bridge_overflow_drops_whole_frames_never_splits_one(), bridge_ring_capacity(), bridge_stays_frame_aligned_across_overflow_rounds(), channel_peak(), ignores_extra_plugin_channels(), interleave_outputs() (+14 more)

### Community 38 - "Interfaz de Arreglo"
Cohesion: 0.07
Nodes (34): applyArrangementTrackHeights(), arrangementTrackHeight(), arrangementTrackHeightKey(), arrangementTrackHeights, audioInputDevices, audioReturnDevices, browserFavorites, browserHistory (+26 more)

### Community 39 - "Comandos e Historial del Proyecto"
Cohesion: 0.22
Nodes (28): add_midi_note_is_reversible_and_preserves_existing_expression_events(), adding_audio_track_assigns_stereo_channels_and_default_master_output_reversibly(), attach_midi_take_is_serializable_and_emits_an_attributed_event(), audio_region_commands_add_trim_gain_and_fades_undo_without_changing_source(), audio_source_proxy_uses_reversible_command_and_checks_provenance(), command_envelope_has_a_stable_serializable_contract(), Midi, CommandRuntime (+20 more)

### Community 40 - "Procesamiento DSP y Ecualización"
Cohesion: 0.10
Nodes (23): BiquadCoefficients, BiquadState, calculate_coefficients(), disabled_band_is_transparent(), DspError, InvalidBandIndex, InvalidBandParameter, InvalidBlockLength (+15 more)

### Community 41 - "Lectura de Metadatos del Plugin"
Cohesion: 0.09
Nodes (32): accepts_comments_and_trailing_commas_without_touching_strings(), bundle_root(), checked_i32(), CURRENT_UID, exposes_validated_moduleinfo_snapshot_paths(), find_path(), MAX_CLASSES, MAX_COMPATIBILITY_ENTRIES (+24 more)

### Community 42 - "Seguridad y Autorización de Plugins"
Cohesion: 0.11
Nodes (24): authorize_external_code(), bundle_identity_changes_when_member_contents_change(), file_identity_changes_when_contents_change(), hash_file(), hash_path(), identify(), load_registry(), local_wine_prefix() (+16 more)

### Community 43 - "Buffers de Audio por Bus"
Cohesion: 0.13
Nodes (21): AudioBusBuffer, actually_zero_output_is_marked_silent_even_when_plugin_omits_the_flag(), build_typed_sample_buffers(), bus_buffers_route_active_buses_without_flattening_inactive_slots(), buses(), channel_mask(), copy_bus_inputs_to(), copy_bus_outputs_to() (+13 more)

### Community 44 - "Worker de FluidSynth"
Cohesion: 0.11
Nodes (17): command_queue(), effective_queue_target_frames(), failed_soundfont_preparation_keeps_the_active_render_plan_playable(), FluidSynthPcmNode, FRAMES_PER_CHUNK, midi_command_queue_is_ordered_bounded_and_nonblocking(), MIDI_QUEUE_CAPACITY, paused_worker_keeps_soundfont_voice_and_pcm_queue_stationary() (+9 more)

### Community 45 - "Streams de Estado VST3"
Cohesion: 0.09
Nodes (16): a_restore_stream_publishes_the_context_it_was_built_from(), create_memory_stream_from_with_metadata(), create_memory_stream_with_metadata(), create_state_restore_stream(), MemBuf, MemoryStream, read_attribute(), stream_attributes_expose_bounded_filename_and_state_type() (+8 more)

### Community 46 - "Enlace con FluidSynth"
Cohesion: 0.10
Nodes (25): ChannelValue, ControlChange, DeleteSettings, DeleteSynth, FLUIDSYNTH_TEST_LOCK, FluidSynthApi, RuntimeUnavailable, KeyValue (+17 more)

### Community 47 - "Grabación MIDI"
Cohesion: 0.10
Nodes (26): convert_event(), LiveRecordError, Alsa, Discovery, Recorder, MidiRecorder, note_on(), preserves_pressure_and_source() (+18 more)

### Community 48 - "Motor FluidSynth"
Cohesion: 0.08
Nodes (18): FluidSynthEngine, FluidSynthError, BlockTooLarge, InvalidSampleRate, MidiEvent, MissingSymbol, NonUtf8Path, NoPresets (+10 more)

### Community 49 - "Host de Audio y Runtime"
Cohesion: 0.11
Nodes (9): AudioProfileSettings, audio_clip_duration_session_ticks(), AudioRuntimeHost, FinishedInputRecording, invalid_project_play_error_does_not_publish_connected_audio_state(), micros_to_session_ticks(), next_quantized_tick(), process_has_executable() (+1 more)

### Community 50 - "Transporte y Coordinación MIDI"
Cohesion: 0.12
Nodes (23): TransportLoopRange, apply_scheduler_command(), coordinate_loop(), EventSender, Alsa, Sine, SoundFont, Vst3 (+15 more)

### Community 51 - "Pruebas de Asignación en Tiempo Real"
Cohesion: 0.10
Nodes (7): record_allocation_on_this_thread(), render_plan_handoff_at_audio_boundary_does_not_allocate(), TEST_ALLOCATOR, ThreadLocalTrackingAllocator, ALLOCATOR, CountingAllocator, record_allocation()

### Community 52 - "Descubrimiento de Plugins VST3"
Cohesion: 0.11
Nodes (26): BusInfo, BusLayout, ClassInfo, DEFAULT_PROBE_TIMEOUT, DetailedPluginInfo, discover_plugins_safe(), FactoryInfo, find_probe_binary() (+18 more)

### Community 54 - "Aplicación e Historial del Proyecto"
Cohesion: 0.12
Nodes (18): ApplicationError, Json, NoProjectPath, ProjectJson, Queue, Rejected, UnexpectedApplyCount, open_execute_undo_redo_and_atomic_save_round_trip() (+10 more)

### Community 55 - "Interfaz de Arreglo y Clips"
Cohesion: 0.12
Nodes (32): addTrack(), arrangementTrackOrder(), bindAudioRegionEditing(), bindClipSelection(), bindMidiClipMovement(), createMidiClipAtCursor(), createSessionMidiClip(), createTrackRemovalButton() (+24 more)

### Community 56 - "Integración de Ventanas macOS"
Cohesion: 0.07
Nodes (5): MacEmbed, rect(), rejects_a_non_finite_position_even_with_a_valid_size(), rejects_sizes_a_window_system_cannot_represent(), WAYLAND_UNSUPPORTED

### Community 57 - "Gestión del Editor Embebido"
Cohesion: 0.11
Nodes (9): EditorRect, EmbeddedEditor, LinuxEmbed, NegotiatedSize, rejects_wayland_surface_with_actionable_contract_error(), try_lock(), validated_size(), WinEmbed (+1 more)

### Community 58 - "CLI de Audio y PipeWire"
Cohesion: 0.10
Nodes (23): PipeWireTargets, audio_record_command(), audio_test_command(), audiobox_targets(), build_synth_render_plan(), capture_config(), capture_config_uses_the_selected_period_for_normal_and_monitor_streams(), midi_synth_live_command() (+15 more)

### Community 59 - "Control del Stream de Audio"
Cohesion: 0.11
Nodes (13): AudioStream, RtAudioHandle, is_normalized(), OwnerThreadTeardown<T>, RtCommand, Midi, Param, Transport (+5 more)

### Community 60 - "Carga de Bundles macOS"
Cohesion: 0.12
Nodes (8): BundleEntryFunc, BundleExitFunc, falls_back_to_scanning_when_the_plist_declares_nothing(), fixture_bundle(), GetPluginFactoryFunc, prefers_the_executable_named_by_cfbundleexecutable(), PrivateNamespaceModule, PrivateNamespaceModuleLoader

### Community 61 - "Host VST3 y Entrada MIDI"
Cohesion: 0.11
Nodes (11): bind_to_handle(), CLIENT_NAME, connect(), list_midi_input_ports(), list_midi_input_ports_does_not_error(), midi_err(), MidiInputConnection, MidiInputPort (+3 more)

### Community 62 - "Nodos de Audio DSP"
Cohesion: 0.11
Nodes (17): AudioNode, AudioNodeError, Dsp, InvalidBlockLength, WorkerFailed, EqualizerNode, GainNode, ScaleNode (+9 more)

### Community 63 - "Controles MIDI de Transporte"
Cohesion: 0.11
Nodes (19): MidiBinding, MidiControlAction, MasterVolume, NextScene, PreviousScene, Stop, ToggleLoop, TogglePlay (+11 more)

### Community 64 - "Diagnóstico de Dispositivos MIDI"
Cohesion: 0.14
Nodes (16): midi_monitor_command(), midi_outputs_command(), DeviceInfo, DiagnosticsError, Alsa, Command, Json, PipeWire (+8 more)

### Community 66 - "Control del Plugin de Audio"
Cohesion: 0.14
Nodes (8): AudioHandle, HybridCommand, Midi, Panic, Param, Transport, MidiSink, queue_command()

### Community 67 - "Carga de Plugins macOS"
Cohesion: 0.11
Nodes (8): bundle_executable_archs(), BundleEntryFunc, BundleExitFunc, GetPluginFactoryFunc, HOST_ARCH, MacOSModule, MacOSModuleLoader, parse_macho_archs()

### Community 69 - "Comandos de Clips de Audio"
Cohesion: 0.17
Nodes (23): add_audio_clip(), add_audio_clip_for_source(), append_media_source(), append_media_source_with_id(), attach_media_source(), attaches_media_source_only_to_audio_tracks_and_survives_json(), AudioClipError, ClipNotFound (+15 more)

### Community 70 - "Colas de Automatización VST3"
Cohesion: 0.16
Nodes (6): active_points_can_be_drained_into_bounded_feedback_before_clear(), enqueue_groups_by_id_orders_by_offset_and_clears(), enqueue_with_nan_value_orders_by_offset_without_panicking(), output_queue_is_isolated_per_block_and_does_not_grow_when_cleared(), ParameterChanges, ParameterValueQueue

### Community 71 - "Curvas de Automatización"
Cohesion: 0.15
Nodes (16): add_point_with_nan_time_does_not_panic(), add_point_with_nan_value_is_not_used_in_ordering(), AutomationCurve, Exponential, Linear, Logarithmic, Step, AutomationPoint (+8 more)

### Community 72 - "Entrada MIDI ALSA"
Cohesion: 0.15
Nodes (6): create_port(), open_seq_client(), receive_note(), receiver_can_resume_after_source_port_disconnect_and_reconnect(), send_note(), subscribe()

### Community 73 - "Atributos del Host VST3"
Cohesion: 0.21
Nodes (9): attr_key(), attribute_list_round_trips_each_type(), AttrValue, Bin, Float, Int, Str, HostAttributeList (+1 more)

### Community 74 - "Conexión Componente-Controlador"
Cohesion: 0.21
Nodes (3): ConnectionPair, InitializedComponent, terminate_component()

### Community 75 - "Carga de Módulos VST3"
Cohesion: 0.13
Nodes (9): LinuxModuleLoader, ModuleLoader, VstModule, arch_mismatch_detail(), ExitDllFunc, GetPluginFactoryFunc, InitDllFunc, WindowsModule (+1 more)

### Community 76 - "Estado y Audio del Plugin VST3"
Cohesion: 0.12
Nodes (16): PluginStateReference, load_plugin_state_bytes(), midi_channel(), plugin_state_relative_path(), SessionClipKind, Audio, Midi, vst3_worker_returns_real_plugin_audio_outside_the_render_callback() (+8 more)

### Community 77 - "Controles de Cabecera de Pista"
Cohesion: 0.13
Nodes (22): createArrangementTrackHeader(), createTrackGroupToolbar(), createTrackHeadArm(), createTrackHeadGain(), createTrackHeadToggle(), createTrackIdentityControl(), createTrackInputControl(), createTrackMeter() (+14 more)

### Community 78 - "Notificaciones del Host VST3"
Cohesion: 0.09
Nodes (13): ContextMenuItem, HostNotification, ContextMenuRequested, DirtyChanged, GroupEditFinished, GroupEditStarted, OpenEditorRequested, ProgramListChanged (+5 more)

### Community 79 - "Detección de Arquitectura de Plugin"
Cohesion: 0.10
Nodes (13): ELF_MACHINE_386, ELF_MACHINE_AARCH64, ELF_MACHINE_X86_64, HOST_ARCH, load_module(), mismatch_detail(), mismatch_detail_is_actionable(), pe_fixture() (+5 more)

### Community 80 - "Mezcla de Audio de Sesión"
Cohesion: 0.23
Nodes (12): AudioClipMixerNode, AudioClipStream, returning_to_arrangement_discards_queued_session_audio_before_unmuting_it(), Replace, session_clear_retires_maximum_active_groups_without_releasing_on_callback(), session_command_queue_reports_full_without_waiting_or_dropping_in_callback(), session_commands_transfer_and_reclaim_outside_audio_node(), SessionAudioCommand (+4 more)

### Community 81 - "Proxy de Conexión VST3"
Cohesion: 0.17
Nodes (6): connection_proxy_forwards_only_on_its_control_thread(), ConnectionProxy, forwarded_and_null_notifies_do_not_count_as_off_thread_drops(), off_thread_notifies_are_counted_so_the_drop_is_diagnosable(), proxy_with_destination(), RecordingConnectionPoint

### Community 83 - "Configuración de Buses del Plugin"
Cohesion: 0.16
Nodes (10): AudioBusConfig, AudioBusLayout, a_context_free_state_restore_still_says_project(), a_state_restore_carries_its_context_across_the_boundary(), FAKE_HELPER_TIMEOUT, FakeHelper, isolated(), MAX_OUTPUT_MIDI (+2 more)

### Community 84 - "Menús Contextuales VST3"
Cohesion: 0.15
Nodes (5): ContextMenuRegistry, HostContextMenu, PendingContextMenuItem, StoredContextMenuItem, TestContextMenuTarget

### Community 85 - "Carga y Reproducción de Plugins"
Cohesion: 0.16
Nodes (14): PluginInfo, discover_plugins(), discover_plugins_in(), get_plugin_info(), is_valid_plugin(), load_plugin(), load_plugin_isolated(), load_plugin_with_settings() (+6 more)

### Community 86 - "Historial y Eventos del Proyecto"
Cohesion: 0.14
Nodes (8): ChangeSet, project_history_publishes_transactions_and_supports_undo_redo(), ProjectEvent, ProjectEventKind, Committed, Redone, Undone, ProjectHistory

### Community 87 - "Colas de Eventos MIDI de Salida"
Cohesion: 0.15
Nodes (5): drains_in_order_and_drops_oldest_when_full(), handle_is_send_and_shares_the_queue_across_threads(), note(), OutputEventConsumer, OutputMidiConsumer

### Community 88 - "Estado y Presets VST3"
Cohesion: 0.16
Nodes (6): decode_state_snapshot(), encode_state_snapshot(), read_snapshot_u32(), state_snapshot_rejects_bad_lengths_and_versions(), state_snapshot_round_trip_and_legacy_component_compatibility(), StateSnapshot

### Community 89 - "Runtime de Cómputo y GPU"
Cohesion: 0.24
Nodes (14): choose_backend(), ComputePolicy, ComputeRequest, deadline_risk_falls_back(), GpuAdapterReport, insufficient_gpu_memory_falls_back(), policy_can_explicitly_allow_realtime_gpu(), probe_gpu_adapters() (+6 more)

### Community 90 - "Procesamiento de Buffers por Bloques"
Cohesion: 0.19
Nodes (6): BusAudioBuffers, CallerAudioBuffers, Buses, Flat, chunk_offset(), chunk_offset_maps_each_offset_once()

### Community 91 - "Utilidades de Identificadores VST3"
Cohesion: 0.22
Nodes (11): c_str_to_string(), class_uid_matches(), class_uid_matches_for_platform(), class_uid_parser_is_strict_and_uses_native_com_order(), format_class_uid(), format_class_uid_for_platform(), is_class_uid(), parse_class_uid() (+3 more)

### Community 93 - "Errores del Host VST3"
Cohesion: 0.12
Nodes (15): Error, AudioBackendError, InterfaceError, InvalidParameter, IoError, MidiError, NotProcessing, Other (+7 more)

### Community 95 - "Interfaz del Navegador de Plugins"
Cohesion: 0.20
Nodes (15): appendBrowserContentRow(), appendBrowserEmpty(), browserCollectionMatches(), browserItemKey(), createBrowserCollectionButton(), createBrowserFavoriteButton(), createTrackInstrumentControl(), instrumentBackendLabel() (+7 more)

### Community 96 - "Configuración de la Aplicación Tauri"
Cohesion: 0.13
Nodes (14): app, security, windows, withGlobalTauri, build, frontendDist, bundle, active (+6 more)

### Community 97 - "Crates del Dominio y Runtime"
Cohesion: 0.31
Nodes (15): estudio-daw-application, estudio-daw-audio-engine, estudio-daw-audio-platform, estudio-daw-cli, estudio-daw-command-bus, estudio-daw-dsp, estudio-daw-media-adapter, estudio-daw-midi-engine (+7 more)

### Community 98 - "Decodificación de Audio de Sesión"
Cohesion: 0.21
Nodes (6): AudioClip, AudioSource, AudioDecodePump, PreparedSessionAudio, session_audio_streams(), SessionLaunch

### Community 99 - "Eventos MIDI del Worker"
Cohesion: 0.16
Nodes (12): apply_midi_event(), WorkerCommand, Midi, Stop, SynthMidiEvent, ChannelPressure, ControlChange, KeyPressure (+4 more)

### Community 100 - "Linux Plugin Loading"
Cohesion: 0.21
Nodes (5): arch_mismatch_detail(), GetPluginFactoryFunc, LinuxModule, ModuleEntryFunc, ModuleExitFunc

### Community 101 - "Plugin Helper Recovery"
Cohesion: 0.22
Nodes (6): a_crash_on_any_other_command_is_reported_not_retried(), a_load_that_always_crashes_gives_up_after_one_retry(), a_load_that_kills_the_helper_is_replayed_against_a_fresh_one(), explicit_helper_override_missing_path_reports_clearly(), FlakyLoadHelper, hung_helper_times_out_and_is_killed_not_blocking()

### Community 102 - "IPC Exchange Errors"
Cohesion: 0.20
Nodes (7): ExchangeError, AlreadyDead, DiedDuringCommand, Encoding, TimedOut, is_slow_command(), String

### Community 103 - "COM Host Messaging"
Cohesion: 0.23
Nodes (3): create_host_attribute_list(), create_host_message(), HostMessage

### Community 104 - "Compute Backend Selection"
Cohesion: 0.18
Nodes (11): ComputeBackend, Cpu, Gpu, ComputeDecision, FallbackReason, DeadlineRisk, GpuMemoryInsufficient, GpuUnavailable (+3 more)

### Community 105 - "MIDI Clip Editing"
Cohesion: 0.18
Nodes (12): create_empty_midi_clip(), creates_empty_midi_clip_at_cursor_with_one_bar_duration(), MidiEditError, ClipNotFound, DuplicateClipId, InvalidDuration, InvalidGrid, InvalidName (+4 more)

### Community 106 - "Plugin Compatibility Validation"
Cohesion: 0.29
Nodes (8): ClassCompatibility, discover_snapshots(), invalid(), normalize_uid(), read_factory_compatibility(), resolve_factory_audio_class_id(), validate_compatibility(), validate_string()

### Community 107 - "Denormal Number Handling"
Cohesion: 0.18
Nodes (6): DAZ, FTZ, FZ, Guard, guard_flushes_denormals_to_zero(), make_denormal()

### Community 108 - "Audio Codec Serialization"
Cohesion: 0.18
Nodes (10): audio_codec_round_trips_and_shrinks_the_payload(), clamped_bus_count(), clamped_channel_count(), decode(), decode_channel(), deserialize(), encode_channel(), serialize() (+2 more)

### Community 109 - "Live Transport State"
Cohesion: 0.20
Nodes (6): applies_midi_commands_to_live_session_state(), LiveSessionState, TransportState, Paused, Playing, Stopped

### Community 110 - "Live ALSA MIDI"
Cohesion: 0.22
Nodes (7): LiveControlError, Alsa, Discovery, Discovery, run_alsa_midi_control(), run_alsa_midi_input_until(), live_input_subscribes_receives_a_note_and_stops()

### Community 111 - "Clip Launch Quantization"
Cohesion: 0.18
Nodes (11): ClipLaunchMode, Loop, OneShot, ClipLaunchQuantization, Bar, Eighth, Global, Immediate (+3 more)

### Community 112 - "Workstation UI Actions"
Cohesion: 0.25
Nodes (11): actionTarget(), closeContextMenu(), contextTrack(), executeUiAction(), handleWorkstationShortcut(), moveTrackBy(), refreshActionAvailability(), renderApplicationMenu() (+3 more)

### Community 115 - "Plugin Bus Arrangements"
Cohesion: 0.18
Nodes (6): BusArrangements, SpeakerArrangement, .EMPTY, .MONO, .STEREO, .STEREO_SURROUND

### Community 116 - "Plugin Event Data"
Cohesion: 0.18
Nodes (11): PluginEventData, Chord, Data, LegacyMidiCcOut, NoteExpressionIntValue, NoteExpressionText, NoteExpressionValue, NoteOff (+3 more)

### Community 117 - "Recorded MIDI Messages"
Cohesion: 0.20
Nodes (10): RecordedMidiMessage, ChannelPressure, ControlChange, KeyPressure, NoteOff, NoteOn, Other, PitchBend (+2 more)

### Community 119 - "Plugin Snapshot Discovery"
Cohesion: 0.25
Nodes (6): discover_plugin_snapshots(), ModuleClassInfo, ModuleFactoryFlags, ModuleFactoryInfo, ModuleInfo, PluginSnapshot

### Community 122 - "Audio Compute Benchmarks"
Cohesion: 0.38
Nodes (3): benchmark_convolution(), benchmark_fft(), main()

### Community 123 - "Compute Workload Types"
Cohesion: 0.29
Nodes (7): WorkloadKind, Analysis, Convolution, Fft, OfflineRender, RealtimeAudio, TimeStretch

### Community 124 - "Session Clip Launching"
Cohesion: 0.48
Nodes (7): focusSessionSceneLaunch(), launchSessionScene(), launchSessionSlot(), renderSessionSurface(), sessionQuantizeLabel(), sessionQuantizeTicks(), stopSessionTrack()

### Community 125 - "Plugin Bus Configuration"
Cohesion: 0.33
Nodes (5): BusDirection, Input, Output, MediaType, Audio

### Community 127 - "Plugin Isolation Errors"
Cohesion: 0.29
Nodes (7): IsolationError, Crashed, Io, NotRunning, Plugin, Serialization, UnexpectedResponse

### Community 128 - "Bounded IPC Line Reading"
Cohesion: 0.40
Nodes (5): read_bounded_line(), ReadLine, Eof, Line, Oversized

### Community 129 - "FFT Implementation"
Cohesion: 0.33
Nodes (6): fft_in_place(), fft_round_trip_recovers_signal(), FftError, EmptyInput, LengthMismatch, LengthNotPowerOfTwo

### Community 130 - "Windows Permissions Configuration"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 131 - "Plugin Probe Outcomes"
Cohesion: 0.47
Nodes (6): ProbeOutcome, Crashed, Failed, Ok, TimedOut, run_probe()

### Community 133 - "Automation Modes"
Cohesion: 0.33
Nodes (5): AutomationState, Off, Read, ReadWrite, Write

### Community 134 - "Plugin Factory Flags"
Cohesion: 0.40
Nodes (5): RawFactoryFlagObject, RawFactoryFlags, Integer, Missing, Object

### Community 135 - "Plugin Process Modes"
Cohesion: 0.40
Nodes (4): ProcessMode, Offline, Prefetch, Realtime

### Community 136 - "Audio Preview Controls"
Cohesion: 0.67
Nodes (4): playPreviewBytes(), previewAudio(), previewPendingAudio(), stopPreview()

### Community 139 - "Plugin Runtime Status"
Cohesion: 0.50
Nodes (4): PluginStatus, Crashed, Ok, Timeout

### Community 141 - "Plugin State Encoding"
Cohesion: 0.67
Nodes (3): StateWire, Base64, Legacy

## Knowledge Gaps
- **498 isolated node(s):** `NoMidiTrack`, `FluidSynth`, `Sine`, `Standalone`, `Vst3` (+493 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 1267 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **25 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Error` connect `Errores del Host VST3` to `Modelo de Proyecto`, `Adaptador de Medios`, `Plan de Renderizado de Audio`, `Bloques de Audio`, `Formato Binario VST3`, `Sonda y Descubrimiento VST3`, `Implementación del Plugin VST3`, `Bus de Comandos del Proyecto`, `Configuración de la Aplicación`, `Gestión de Ventanas del Plugin`, `Backend de Audio CPAL`, `Sesión y Reloj de Transporte`, `Reproducción MIDI`, `Motor de Síntesis y Mezcla`, `Puente de Audio de Sesión`, `Procesamiento DSP y Ecualización`, `Lectura de Metadatos del Plugin`, `Worker de FluidSynth`, `Enlace con FluidSynth`, `Integración de Ventanas macOS`, `Carga de Bundles macOS`, `Host VST3 y Entrada MIDI`, `Diagnóstico de Dispositivos MIDI`, `Carga de Plugins macOS`, `Carga de Módulos VST3`, `Configuración de Buses del Plugin`, `Carga y Reproducción de Plugins`, `Linux Plugin Loading`?**
  _High betweenness centrality (0.101) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `Plugin` (e.g. with `ParameterUpdate` and `.new()`) actually correct?**
  _`Plugin` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `NoMidiTrack`, `FluidSynth`, `Sine` to the rest of the system?**
  _498 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Modelo de Proyecto` be split into smaller, more focused modules?**
  _Cohesion score 0.039586919104991396 - nodes in this community are weakly interconnected._
- **Why does `PluginImpl` connect `Implementación del Plugin VST3` to `Intercambio de Datos`, `Plugin Process Modes`, `Implementación del Plugin VST3`, `Integración COM del Host VST3`, `Edición de Parámetros VST3`, `Canales MIDI`, `Lista de Eventos VST3`, `Interfaz Interna del Plugin`, `Respuestas del Host VST3`, `Buffers de Audio por Bus`, `Vista del Editor VST3`, `Ciclo de Vida del Host VST3`, `Curvas de Automatización`, `Conexión Componente-Controlador`, `Carga de Módulos VST3`, `Carga y Reproducción de Plugins`, `Colas de Eventos MIDI de Salida`, `Estado y Presets VST3`, `Procesamiento de Buffers por Bloques`, `Plugin Compatibility Validation`?**
  _High betweenness centrality (0.071) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `PluginImpl` (e.g. with `ClassCompatibility` and `PluginUnit`) actually correct?**
  _`PluginImpl` has 3 INFERRED edges - model-reasoned connections that need verification._
- **Should `Adaptador de Medios` be split into smaller, more focused modules?**
  _Cohesion score 0.05759623861298854 - nodes in this community are weakly interconnected._