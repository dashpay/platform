const { Isolate } = require('isolated-vm');
const allocateRandomMemory = require('../../../../lib/test/util/allocateRandomMemory');
const waitShim = require('../../../../lib/test/util/setTimeoutShim');

const invokeSyncFunctionFromIsolate = require('../../../../lib/dpp/isolation/invokeSyncFunctionFromIsolate');

describe('invokeSyncFunctionFromIsolate', function describe() {
  let isolate;
  let context;
  let jail;

  this.timeout(4000);

  beforeEach(async () => {
    isolate = new Isolate({ memoryLimit: 10 });

    context = await isolate.createContext();

    ({ global: jail } = context);

    await jail.set('global', jail.derefInto());

    await context.eval(`global.wait = ${waitShim}`);

    await context.eval(`
      global.infiniteLoop = function infiniteLoop() {
        while(true) {}
        return;
      };
    `);

    await context.eval(`
      global.allocateRandomMemory = ${allocateRandomMemory}
    `);
  });

  afterEach(() => {
    if (!isolate.isDisposed) {
      isolate.dispose();
    }
  });

  it('should stop execution after a timeout for an async function', () => {
    const timeout = 300;

    const timeStart = Date.now();

    let error;
    try {
      invokeSyncFunctionFromIsolate(
        context,
        '',
        'wait',
        [5000],
        { timeout },
      );
    } catch (e) {
      error = e;
    }

    const timeSpent = Date.now() - timeStart;

    expect(error).to.be.instanceOf(Error);
    expect(error.message).to.be.equal('Script execution timed out.');
    expect(timeSpent >= timeout).to.be.true();
    expect(timeSpent).to.be.lessThan(timeout + 100);
  });

  it('should stop execution after a timeout for a sync function', () => {
    const timeout = 300;
    const timeStart = Date.now();

    let error;
    try {
      invokeSyncFunctionFromIsolate(
        context,
        '',
        'infiniteLoop',
        [],
        { timeout },
      );
    } catch (e) {
      error = e;
    }

    const timeSpent = Date.now() - timeStart;

    expect(error).to.be.instanceOf(Error);
    expect(error.message).to.be.equal('Script execution timed out.');

    expect(timeSpent >= timeout).to.be.true();
    expect(timeSpent).to.be.lessThan(timeout + 1000);
  });

  it('should not stop execution if memory utilization is less than limit', () => {
    invokeSyncFunctionFromIsolate(
      context,
      '',
      'allocateRandomMemory',
      // 5 mb should be fine, as the limit set in beforeEach hook is 10
      [5 * 1000 * 1000],
    );
  });

  it('should stop execution if memory is exceeded', () => {
    let error;
    try {
      invokeSyncFunctionFromIsolate(
        context,
        '',
        'allocateRandomMemory',
        // 15 mb, while our limit is 10 mb
        [15 * 1000 * 1000],
      );
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(Error);
    expect(error.message).to.be.equal('Isolate was disposed during execution due to memory limit');
  });
});
