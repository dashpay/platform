const ExecutionTimer = require('../../../lib/util/ExecutionTimer');
const wait = require('../../../lib/util/wait');

describe('ExecutionTimer', () => {
  let timer;

  beforeEach(() => {
    timer = new ExecutionTimer();
  });

  describe('#startTimer', () => {
    it('should throw an error if timer already started', () => {
      timer.startTimer('some');

      try {
        timer.startTimer('some');
        expect.fail('An error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('some timer is already started');
      }
    });
  });

  describe('#endTimer', () => {
    it('should throw an error if timer has not been started', () => {
      try {
        timer.endTimer('some');
        expect.fail('An error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('some timer is not started');
      }
    });
  });

  it('should measure function execution time', async () => {
    // TODO: maybe there should be a better way to do it
    timer.startTimer('some');
    await wait(1500);
    const timings = timer.endTimer('some');

    expect(parseInt(timings)).to.equal(1);
  });
});
