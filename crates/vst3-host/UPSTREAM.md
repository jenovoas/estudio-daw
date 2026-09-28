# Procedencia del host VST3

Este crate contiene `vst3-host` 0.9.0, obtenido del paquete publicado en
crates.io y mantenido originalmente en
https://github.com/HelgeSverre/rust-vst3-host/ . Su licencia es MIT; el texto
original está en `LICENSE`. Se conserva la API pública y el código upstream.
El backend CPAL de la distribución original se excluyó de este fork porque
usa `alsa-sys` 0.4, que no puede coexistir en el mismo grafo Cargo con el
`alsa-sys` 0.3.1 requerido por el backend MIDI del proyecto. Este producto usa
PipeWire como dispositivo y el helper del crate sólo procesa bloques; por eso
el backend CPAL del crate no se necesita.

Parche local: durante lectura de `IPluginCompatibility`, si el factory enumera
esa clase pero no puede crear la interfaz, se trata como metadato suplementario
ausente y se permite continuar con las clases de audio normales. La conducta se
observó con Analog Lab V 5.12.5.6878 detrás de yabridge 5.1.1 el 2026-09-28.
El parche debe conservar una regresión y revisarse contra versiones upstream
antes de actualizar el crate.
