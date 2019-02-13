const { mocha: { startDashCore } } = require('@dashevo/js-evo-services-ctl');
const isDashCoreRunningFactory = require('../../../lib/sync/isDashCoreRunningFactory');

const wait = require('../../../lib/util/wait');

describe('IsDashCoreRunning', function main() {
  this.timeout(10000);

  let dashCoreApi;
  let isDashCoreRunning;

  startDashCore().then((dashCore) => {
    dashCoreApi = dashCore.getApi();
  });

  beforeEach(() => {
    isDashCoreRunning = isDashCoreRunningFactory(dashCoreApi);
  });

  it('should return true if DashCore is running', async () => {
    const isRunning = await isDashCoreRunning();
    expect(isRunning).to.be.true();
  });

  it('should return false if DashCore is down', async () => {
    await dashCoreApi.stop();

    await wait(1000);

    const retries = null;
    const retryDelay = 0.1;
    const isRunning = await isDashCoreRunning(retries, retryDelay);
    expect(isRunning).to.be.false();
  });
});
