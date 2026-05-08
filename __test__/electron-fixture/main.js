// Sentinels guard against Electron startup noise on stdout.
const { app } = require('electron');
const path = require('node:path');

app.disableHardwareAcceleration();

app
  .whenReady()
  .then(async () => {
    const distEntry = path.resolve(__dirname, '..', '..', 'dist', 'index.cjs');
    const { getNotificationStatus } = require(distEntry);

    const status = await getNotificationStatus();

    process.stdout.write('===NOTIFY_STATUS_BEGIN===\n');
    process.stdout.write(JSON.stringify(status) + '\n');
    process.stdout.write('===NOTIFY_STATUS_END===\n');

    app.exit(0);
  })
  .catch((err) => {
    process.stderr.write(
      `[electron-fixture] error: ${err && err.stack ? err.stack : String(err)}\n`,
    );
    app.exit(1);
  });
