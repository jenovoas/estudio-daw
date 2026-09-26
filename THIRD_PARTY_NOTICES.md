# Third-party components

Estudio DAW does not bundle FluidSynth or SoundFont banks. The FluidSynth
adapter loads the host's shared library at runtime through `libloading`.

| Component | Use | License / distribution |
|---|---|---|
| FluidSynth | Optional local SoundFont synthesis runtime | LGPL-2.1-or-later; dynamically loaded from the user's system, not vendored here. The project source and license are available at [FluidSynth](https://github.com/FluidSynth/fluidsynth). |
| libloading | Loads the optional FluidSynth shared library | ISC license; dependency metadata and license are recorded in `Cargo.lock` and the crate registry package. |
| SoundFont files (`.sf2`) | User-selected instrument banks | Not included or downloaded by Estudio DAW. Users must obtain banks from a source whose license permits their intended use and retain the accompanying license/attribution. |

This notice is an inventory, not legal advice. Any future packaging that ships
FluidSynth or a SoundFont must include the corresponding license texts and
comply with the library/bank redistribution terms of that exact version.

## libloading — ISC license

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
