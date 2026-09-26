# MediaSource y proxies

El proyecto conserva siempre el archivo original y trata el proxy como una
representación derivada y reemplazable. `MediaSource` registra la ruta original,
su firma de procedencia y un `ProxyAsset` opcional con perfil y firma de la
fuente que lo generó.

## Resolución

`ProxyPolicy` define la intención del consumidor:

- `Original`: exige que exista el original.
- `Proxy`: exige un proxy existente cuya firma coincida con la fuente.
- `Auto`: usa el proxy válido y vuelve al original cuando todavía no hay proxy.

La resolución sólo selecciona una ruta; no copia, modifica ni re-encodea ningún
archivo. Si el original cambió, la firma del proxy deja de coincidir y el
proxy se considera obsoleto. La firma inicial es una comprobación barata de
tamaño y fecha de modificación; el job de generación añadirá hash de contenido
para proyectos que necesiten invalidación fuerte.

## Frontera futura

La siguiente capa será `ProxyJob`: leerá `MediaSource`, generará el perfil
solicitado en caché, validará duración/canales/sample rate y publicará un nuevo
`ProxyAsset` atómicamente. El editor podrá seguir usando la representación
anterior mientras el job trabaja, y el render final resolverá explícitamente
`Original`.
