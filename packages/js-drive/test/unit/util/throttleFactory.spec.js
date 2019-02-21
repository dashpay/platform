const throttleFactory = require('../../../lib/util/throttleFactory');
const wait = require('../../../lib/util/wait');

describe('throttleFactory', () => {
  let func;
  let throttle;

  beforeEach(function beforeEach() {
    func = this.sinon.stub();
    throttle = throttleFactory(func);
  });

  it('should call function and return result', async () => {
    await throttle(func);

    expect(func).to.have.been.calledOnce();
  });

  it('should call function again if was called during progress only when the first call ended', async () => {
    func.callsFake(async () => {
      await wait(50);
    });

    throttle();

    await wait(10);

    throttle();

    expect(func).to.have.been.calledOnce();

    await wait(100);

    expect(func).to.have.been.calledTwice();
  });

  it('should be callable after function throws an error', async () => {
    const error = new Error();
    func.throws(error);

    let expectedError;
    try {
      await throttle(func);
    } catch (e) {
      expectedError = e;
    }

    expect(expectedError).to.equal(error);
    expect(func).to.have.been.calledOnce();

    func.resetBehavior();

    await throttle(func);

    expect(func).to.have.been.calledTwice();
  });
});
