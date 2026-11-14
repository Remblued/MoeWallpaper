# Privacy policy

**MoeWallpaper does not collect personal data.** There is no account, no telemetry, no analytics,
no crash reporting, and no update check. Nothing is sent to the project's maintainers, because
there is no server of ours to send it to.

## What the app stores on your machine

| What | Where | Why |
| --- | --- | --- |
| Downloaded wallpapers | `%LOCALAPPDATA%\com.remblued.moewallpaper\` (cache) | So setting the same wallpaper twice does not download it twice |
| `current.jpg` | same directory | The file handed to Windows when you set a wallpaper |
| Theme choice | the app's WebView2 profile (local storage) | Remember which of the three themes you picked |
| Files you save | `%USERPROFILE%\Pictures\MoeWallpaper\` | Only when you press "Download" |
| Your own source configs (`*.json`, optional) | `%APPDATA%\com.remblued.moewallpaper\sources\` | Only if you write them. They are plain text files on your disk; the app only reads them, and nothing in them is uploaded |

Deleting `%LOCALAPPDATA%\com.remblued.moewallpaper\` removes the cache and the theme choice;
deleting the folder under `Pictures` removes saved files. Your source configs live in a separate
`%APPDATA%` folder on purpose — clearing the cache never touches them.

If you put a credential (an API key in `headers`) into your own source config, it sits in that
file in plain text. The app does not send it anywhere except to the host you configured.

## What leaves your machine, and to whom

The app fetches images from the sources you select. Those requests go **directly from your
computer to those third-party services** — never through a server of ours:

| Source | Domain |
| --- | --- |
| 360 Wallpaper | `wallpaper.apc.360.cn`, `*.qhimg.com` |
| Safebooru | `safebooru.org` |
| Wallhaven | `wallhaven.cc` |
| nekos.best | `nekos.best` |
| Lolicon | `api.lolicon.app`, `i.pixiv.re` |
| Your own source | whatever host you put in your config — that is your choice, and we never see it |

As with any website you visit, those services can see your IP address, the request time and the
app's user agent (`MoeWallpaper/<version>`). They have their own privacy policies; this project
cannot make promises on their behalf. Nothing else about you is transmitted — no identifier, no
account, no search history stored anywhere except in the app's own window.

If a system proxy is configured in Windows ("Internet Options"), the app uses it, exactly as your
browser would. It is not used when the system proxy is off.

## Children, jurisdiction, changes

No data is collected, so there is nothing age-specific to disclose. This policy applies to the
application itself; GitHub's own policies apply to this repository and to downloads from it.

If this ever changes — for example if an update check is added — it will be written here first,
and the change will be visible in the commit history.
