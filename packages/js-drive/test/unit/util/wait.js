const wait = require('../../../lib/util/wait');

describe('wait', () => {
  let clock;
  let executeWithWait;

  beforeEach(function beforeEach() {
    clock = this.sinon.useFakeTimers();

    executeWithWait = async (f) => {
      await wait(1200);
      f();
    };
  });

  it('should delay execution of a flow for a specified amount of milliseconds', function it(done) {
    const callback = this.sinon.stub();

    executeWithWait(callback).then(() => {
      expect(callback).have.been.calledOnce();
      done();
    }).catch(done);

    clock.tick(1199);

    expect(callback).have.not.been.called();

    clock.tick(1);
  });
});
