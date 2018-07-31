const createDashDriveInstance = require('../../../lib/test/services/dashDrive/createDashDriveInstance');
const createMongoDbInstance = require('../../../lib/test/services/mongoDb/createMongoDbInstance');
const wait = require('../../../lib/test/util/wait');
const { PassThrough } = require('stream');

async function containerLogs(container) {
  const logStream = await container.logs({
    follow: true,
    stdout: true,
    stderr: true,
  });
  let log = '';
  logStream.on('data', (chunk) => {
    log += chunk.toString('utf8');
  });
  const stream = new PassThrough();
  container.modem.demuxStream(logStream, stream, stream);

  await wait(20000);
  logStream.destroy();

  return log;
}

describe('DashDrive throws DashCoreIsNotRunningError', function main() {
  this.timeout(90000);

  let dashDriveInstance;
  let mongoDbInstance;
  beforeEach(async () => {
    mongoDbInstance = await createMongoDbInstance();
    await mongoDbInstance.start();
  });

  it('should throw DashCoreIsNotRunningError if DashCore is not running', async () => {
    const envs = [
      `STORAGE_MONGODB_URL=mongodb://${mongoDbInstance.getIp()}:27017`,
    ];
    dashDriveInstance = await createDashDriveInstance(envs);
    dashDriveInstance.initialize = () => {};

    await dashDriveInstance.start();

    const log = await containerLogs(dashDriveInstance.container.container);
    expect(log.includes('DashCoreIsNotRunningError')).to.be.true();
  });

  after('Clean instances', async () => {
    const promises = Promise.all([
      mongoDbInstance.remove(),
      dashDriveInstance.remove(),
    ]);
    await promises;
  });
});
