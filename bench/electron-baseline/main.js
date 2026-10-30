const { app, BrowserWindow } = require("electron");

// 必须一直持有这个引用。Electron 里 BrowserWindow 的 JS 包装对象一旦被 V8 回收，
// 对应的原生窗口会被自动关掉；接着 window-all-closed 触发 app.quit()，
// 应用就会在跑了几十秒之后"自己退出"——不报错、不留事件日志，非常难查。
let win = null;

app.whenReady().then(() => {
  win = new BrowserWindow({ width: 1000, height: 700 });
  win.loadFile("index.html");
});

app.on("window-all-closed", () => app.quit());
