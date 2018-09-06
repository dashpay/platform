const isDashCoreRunningFactory = require('../../../lib/sync/isDashCoreRunningFactory');
const { mocha: { startDashCore } } = require('js-evo-services-ctl');

describe('IsDashCoreRunning', () => {
  let dashCoreInstance;
  let isDashCoreRunning;

  startDashCore().then((instance) => {
    dashCoreInstance = instance;
  });

  beforeEach(() => {
    isDashCoreRunning = isDashCoreRunningFactory(dashCoreInstance.getApi());
  });

  it('should return true if DashCore is running', async () => {
    const isRunning = await isDashCoreRunning();
    expect(isRunning).to.be.true();
  });

  it('should return false if DashCore is down', async () => {
    await dashCoreInstance.stop();

    const retries = null;
    const retryDelay = 0.1;
    const isRunning = await isDashCoreRunning(retries, retryDelay);
    expect(isRunning).to.be.false();
  });
});
