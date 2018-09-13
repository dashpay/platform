const proxyquire = require('proxyquire');

describe('isDashCoreRunningFactory', () => {
  let isDashCoreRunning;
  let rpcClient;
  let wait;
  beforeEach(function beforeEach() {
    wait = this.sinon.spy();
    const isDashCoreRunningFactory = proxyquire('../../../lib/sync/isDashCoreRunningFactory', {
      '../util/wait': wait,
    });
    rpcClient = { ping: this.sinon.stub() };
    isDashCoreRunning = isDashCoreRunningFactory(rpcClient);
  });

  it('should return false, retry 2 times and wait 1 if DashCore is not running', async () => {
    rpcClient.ping.throws(new Error());

    const retries = 2;
    const retryDelay = 0.1;
    const isRunning = await isDashCoreRunning(retries, retryDelay);

    expect(isRunning).to.be.false();
    expect(rpcClient.ping).to.be.calledThrice();
    expect(wait).to.be.calledTwice();
  });

  it('should not wait and return true if DashCore is running', async () => {
    const retries = 2;
    const retryDelay = 0.1;
    const isRunning = await isDashCoreRunning(retries, retryDelay);

    expect(isRunning).to.be.true();
    expect(rpcClient.ping).to.be.calledOnce();
    expect(wait).have.not.been.called();
  });
});
