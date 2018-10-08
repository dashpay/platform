const {
  createDriveApi,
  createDriveSync,
} = require('@dashevo/js-evo-services-ctl');

const wait = require('../../../lib/util/wait');

describe('DashDrive throws DashCoreIsNotRunningError', function main() {
  this.timeout(120000);

  let driveApi;
  let driveSync;

  before(async () => {
    const envs = [
      'DASHCORE_RUNNING_CHECK_MAX_RETRIES=0',
      'DASHCORE_RUNNING_CHECK_INTERVAL=0',
    ];
    const opts = { container: { envs } };
    driveApi = await createDriveApi(opts);
    driveApi.initialize = () => {};
    driveSync = await createDriveSync(opts);
    driveSync.initialize = () => {};

    await Promise.all([
      driveApi.start(),
      driveSync.start(),
      wait(20000), // Waiting for `npm i`
    ]);
  });

  it('API should throw DashCoreIsNotRunningError if DashCore is not running', async () => {
    const log = await driveApi.container.container.logs({ stderr: true });
    expect(log.includes('DashCoreIsNotRunningError')).to.be.true();

    const inspection = await driveApi.container.inspect();
    expect(inspection.State.Running).to.be.false();
  });

  it('Sync should throw DashCoreIsNotRunningError if DashCore is not running', async () => {
    const log = await driveSync.container.container.logs({ stderr: true });
    expect(log.includes('DashCoreIsNotRunningError')).to.be.true();

    const inspection = await driveSync.container.inspect();
    expect(inspection.State.Running).to.be.false();
  });

  after('Clean instances', async () => {
    const instances = [
      driveApi,
      driveSync,
    ];

    // Workaround for "container already stopped"
    await Promise.all(instances.filter(i => i)
      .map((i) => {
        const container = i.container.docker.getContainer(i.container.container.id);
        return container.remove({ v: 1 });
      }));
  });
});
