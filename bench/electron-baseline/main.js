const { app, BrowserWindow } = require("electron");
app.whenReady().then(() => {
  new BrowserWindow({ width: 1000, height: 700 }).loadFile("index.html");
});
app.on("window-all-closed", () => app.quit());
