# 009 — `SettingsService` del engine e idioma persistido

> **Estado:** in-progress — 2026-11-16 — implementación de E10 del change `gafira-engine` ([carpeta legacy](done/legacy/changes/gafira-engine/design.md)).

## Objetivo

Que el engine guarde y lea los ajustes de la persona, empezando por el idioma, y publique `LocaleChanged` al arrancar y en cada cambio de idioma, sin reiniciar (02 §5.4 línea 1364, §5.2 línea 1333, §14.4; RF-66; ARQ-101 en su parte de engine; D-08).

## Alcance / no alcance

Alcance:

- `gafira-ports`: módulo `settings` con `Settings`, `SettingsError` y el puerto `SettingsRepository` (02 §5.3 lo nombra sin firma).
- `gafira-engine`: `SettingsPorts`, `SettingsService::{start, get, set, set_language, locale}`, `Gafira::settings()`, `EngineError::Settings`.

No alcance:

- `gafira-i18n` (`D08LocaleResolver`, catálogos) y `gafira-platform` (`SystemLocaleSource`): el engine recibe el `LocaleResolver` como puerto.
- El adaptador real de `SettingsRepository` (archivo en el directorio de datos): capa 4.
- La UI de Ajustes → General (03 §16.1) y el bench `bench_locale_switch` (CAL-36).
- Otros ajustes además del idioma: `Settings` es `#[non_exhaustive]` para crecer.

## Dependencias

- E3d del change `gafira-engine` (integrado en `develop`).
- `gafira_ports::i18n::{LanguagePreference, LocaleResolver, ResolvedLocale}` (ya existen).
- `EngineEvent::LocaleChanged { preference, resolved }` (ya existe, E1).
- ADR 0016 (engine puro): el engine no lee archivos ni el entorno; todo pasa por puertos.

Decisión de diseño tomada aquí (la spec 02 no fija la firma del puerto): `SettingsRepository` carga y guarda un `Settings` entero; un almacén ausente es `Settings::default()`, no un error. Se agrupa con el `LocaleResolver` en `SettingsPorts` para que `Ports::new` no supere los 7 argumentos de clippy.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S9-01 | `Settings` se serializa como `{"language": "system" \| "es" \| "en"}` (02 §14.4). | `"system"`, `"es"`, `"en"` se leen como su variante y se escriben igual; `"ES"`, `"fr"`, `""` se leen como `System`; `1` y `null` son error de lectura; sin la clave `language` se lee `System`. |
| S9-02 | `SettingsError` tiene clave de mensaje por variante (ARQ-102). | `Corrupt` → `error.settings.corrupt`, `Io` → `error.settings.io`; `EngineError::Settings` delega en ella. |
| S9-03 | `start` carga los ajustes, resuelve el idioma y publica `LocaleChanged` una vez (02 §5.2 línea 1333). | Con `"es"` guardado, publica `LocaleChanged { preference: Es, resolved }` y `get().language == Es`. |
| S9-04 | Si la carga falla, el engine sigue con los valores por defecto. | `start` devuelve `Err(EngineError::Settings(..))`, publica `LocaleChanged` para `System` y `get()` es `Settings::default()`. |
| S9-05 | `set_language` persiste y luego publica (RF-66). | Guarda `language` nuevo, publica `LocaleChanged` con lo que devuelve el resolver y devuelve ese `ResolvedLocale`. |
| S9-06 | Un fallo al guardar no cambia nada. | `set`/`set_language` devuelven `Err`, `get()` y `locale()` siguen igual y no se publica ningún evento. |
| S9-07 | `set` solo publica `LocaleChanged` cuando cambia el idioma. | `set` con el mismo `language` guarda y no publica. |
| S9-08 | El cambio de idioma no toca la sesión (ARQ-101, parte de engine). | Con un documento abierto y un historial de N entradas, `set_language` deja `is_modified` y el historial igual. |

## TDD

Lógica crítica (TDD estricto): S9-01 (lectura y escritura del ajuste persistido) y S9-03..S9-07 (orden persistir → actualizar → publicar). Tests que deben fallar primero:

- `gafira-ports`: `settings::tests::s9_01_*` (tabla de lecturas y escrituras), `s9_02_*`.
- `gafira-engine`: `settings::tests::s9_03_*` a `s9_08_*`, con un `MemorySettings` con fallo inyectable y un resolver falso.

## Evidencia esperada

- Tests anteriores en verde, más los gates de `AGENTS.md` (fmt, clippy `--workspace`, test `--workspace`, `sync_agents.py --check`).
- Banner a `done` y fila en `specs/README.md`.

## Riesgos y decisiones abiertas

- La firma de `SettingsRepository` se fija aquí. Si 02 la define distinta más adelante, se migra en `gafira-ports` y en este servicio.
- RF-65 (idioma del sistema) depende del resolver real (`gafira-i18n`) y no se prueba aquí.
