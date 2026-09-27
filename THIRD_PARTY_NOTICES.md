# Componentes de terceros

Estudio DAW no incluye FluidSynth ni bancos SoundFont. El adaptador FluidSynth
carga en tiempo de ejecución la biblioteca compartida del sistema anfitrión
mediante `libloading`.

| Componente | Uso | Licencia / distribución |
|---|---|---|
| FluidSynth | Entorno local opcional de síntesis SoundFont | LGPL-2.1-or-later; se carga dinámicamente desde el sistema del usuario y no se incluye en este repositorio. El código fuente y la licencia están disponibles en [FluidSynth](https://github.com/FluidSynth/fluidsynth). |
| libloading | Carga la biblioteca FluidSynth compartida opcional | Licencia ISC; los metadatos de dependencia y licencia figuran en `Cargo.lock` y en el paquete del registro de crates. |
| Archivos SoundFont (`.sf2`) | Bancos de instrumentos elegidos por el usuario | Estudio DAW no los incluye ni descarga. Cada usuario debe obtener bancos de una fuente cuya licencia permita el uso previsto y conservar la licencia/atribución correspondiente. |

Este aviso es un inventario, no asesoría legal. Cualquier distribución futura
que incluya FluidSynth o un SoundFont debe incorporar los textos de licencia
correspondientes y cumplir las condiciones de redistribución de la biblioteca o
del banco en esa versión concreta.

## libloading — licencia ISC

El texto de la licencia ISC que sigue se conserva en su idioma original para no
alterar sus términos legales.

Copyright © 2015, Simonas Kazlauskas

Permission to use, copy, modify, and/or distribute this software for any purpose
with or without fee is hereby granted, provided that the above copyright notice
and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND
FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS
OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.

### Traducción informativa al español

Se concede permiso para usar, copiar, modificar y/o distribuir este software
para cualquier propósito, con o sin cargo, siempre que el aviso de derechos de
autor anterior y este aviso de permiso aparezcan en todas las copias.

EL SOFTWARE SE PROPORCIONA «TAL CUAL» Y EL AUTOR RENUNCIA A TODA GARANTÍA,
INCLUIDAS LAS GARANTÍAS IMPLÍCITAS DE COMERCIABILIDAD Y APTITUD PARA UN FIN
DETERMINADO. EN NINGÚN CASO EL AUTOR SERÁ RESPONSABLE DE DAÑOS ESPECIALES,
DIRECTOS, INDIRECTOS O CONSECUENTES, NI DE DAÑO ALGUNO DERIVADO DE LA PÉRDIDA
DE USO, DATOS O GANANCIAS, YA SEA EN UNA ACCIÓN CONTRACTUAL, POR NEGLIGENCIA U
OTRA CONDUCTA ILÍCITA, QUE SURJA DE O ESTÉ RELACIONADA CON EL USO O EL
FUNCIONAMIENTO DE ESTE SOFTWARE.

Esta traducción es informativa. En caso de diferencia, prevalece el texto
original de la licencia ISC incluido arriba.
