# 021 — Menú Ver: modo oscuro

> **Estado:** done — 2026-11-16 — S21-01..S21-02 cubiertos por `gafira-ports` (`tests/settings.rs`) y `gafira-ui` (`editor::tests`); 2801 tests.

## Objetivo

Llevar a la app el modo oscuro aprobado en D-82: Ver → Modo oscuro cambia la interfaz al momento y se recuerda entre sesiones (03 §2.2, §3).

## Alcance / no alcance

Alcance:

- `gafira-ports`: `Settings.appearance: Appearance { Light, Dark }`, `"light"` por defecto; cualquier otro texto se lee como claro (un archivo de una versión nueva nunca falla). `with_appearance`.
- `gafira-ui`: menú Ver con la casilla «Modo oscuro»; el shell aplica los tokens del modo y guarda la elección con `SettingsService::set`; al arrancar se usa la guardada.

No alcance:

- Seguir el modo del sistema operativo; Ajustes → General; reglas, guías, cuadrícula e imán del menú Ver.
- La Pantalla de arranque sigue en claro (`splash.mode`, 03 §7.10).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S21-01 | El ajuste se guarda y se lee. | `appearance` va y vuelve; por defecto y con texto desconocido es claro; un archivo sin la clave es claro. |
| S21-02 | Ver → Modo oscuro llega al shell y persiste. | La casilla pide `Go::Dark(true)`; otro engine sobre el mismo archivo de ajustes lee oscuro. |
