**Shakes and Fidget Bot (Sfboteu)**

Bot that player Shakes & Fidget for you.

Download (latest release)
- https://github.com/alexb231/snf-bot-eu/releases/latest

Usage
- Windows: start sfbot.exe. The tray icon appears, right click -> "Open UI".

Server bind host/port
- Optional `serverConfig.json` next to `sfbot.exe` can override bind settings.
- If `serverConfig.json` does not exist, it is created automatically on start with:
  - `host: localhost`
  - `port: 3000`
- Example:
```json
{
  "host": "0.0.0.0",
  "port": 3000
}
```
- `host` supports: `localhost`, `127.0.0.1`, `0.0.0.0`, `::1`, `::`, `*`.
