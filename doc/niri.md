# Ventana flotante en la esquina superior derecha (niri)

La app ya pide todo lo que puede pedir sola:

- tamaño fijo, `resizable(false)`;
- `app_id = "minish"`;
- nivel de ventana `AlwaysOnTop`;
- un pedido de posición `OuterPosition` a la esquina superior derecha.

En niri eso alcanza para que la ventana **se abra flotante** (niri abre
flotantes las ventanas de tamaño fijo) y quede sobre las demás. Lo que **no**
alcanza es la posición: en Wayland un cliente no puede ubicarse a sí mismo, así
que el pedido de `OuterPosition` lo decide el compositor.

Para fijar la esquina, agregá esta regla en `~/.config/niri/config.kdl`, dentro
del bloque donde ya tenés tus `window-rule`:

```kdl
window-rule {
    match app-id="^minish$"
    open-floating true
    default-floating-position x=24 y=24 relative-to="top-right"
}
```

Después recargá la config con `niri msg action reload-config` (o Mod+Shift+R,
según tu atajo) y volvé a abrir la app.

## Si la posición sigue sin aplicarse

`default-floating-position` tiene reportes de no respetarse en algunas
versiones de niri. Alternativa, una vez abierta la ventana:

```sh
niri msg action move-floating-window --id "$(niri msg --json windows \
  | jq -r '.[] | select(.app_id == "minish") | .id')" \
  -x 2000 -y 24
```

Ojo: `-x`/`-y` son coordenadas absolutas del monitor, no relativas al borde; en
un monitor de 2560 px de ancho y una ventana de 720 px, la esquina superior
derecha con 24 px de margen cae en `x = 2560 - 720 - 24 = 1816`.

## Ver la geometría real de la ventana

```sh
niri msg --json windows | jq '.[] | select(.app_id == "minish")'
```
