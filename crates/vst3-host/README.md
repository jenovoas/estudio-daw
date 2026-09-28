# Host VST3 integrado

Este directorio contiene el host VST3 de upstream `HelgeSverre/rust-vst3-host`
0.9.0. La licencia MIT y los cambios locales están documentados en `LICENSE`
y `UPSTREAM.md`.

Estudio DAW consume la API `vst3_host` para inspección de bundles y carga de
plugins. La salida de audio del plugin se procesa fuera del callback en el
proceso `vst3-host-helper`; el helper y `vst3-host-probe` deben construirse con:

```sh
cargo build -p vst3-host --bins
```

El backend CPAL de upstream no se incluye en este fork: Estudio DAW usa su
adaptador PipeWire y mantiene una sola versión del enlace ALSA en el workspace.

El soporte de editores nativos dentro de procesos auxiliares en Linux y la
persistencia del estado de plugin siguen pendientes.
