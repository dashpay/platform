import CoreService from '../../../src/core/CoreService.js';

describe('CoreService', () => {
  let container;
  let coreService;

  beforeEach(function beforeEach() {
    container = {
      inspect: this.sinon.stub().resolves({ State: { Status: 'running' } }),
      stop: this.sinon.stub().resolves(),
      remove: this.sinon.stub().resolves(),
      kill: this.sinon.stub().resolves(),
    };

    coreService = new CoreService({}, { setTimeout: this.sinon.stub() }, container);
  });

  describe('#stop', () => {
    // Without a timeout of its own, Docker waits for the container's stop
    // timeout, which Compose sets from the service's stop_grace_period. Core
    // flushes its chain database in that time; a kill or a forced removal
    // leaves it to recover on the next start.
    it('should stop Core gracefully, with the grace period of its service', async () => {
      const result = await coreService.stop();

      expect(result).to.be.true();
      expect(container.stop).to.have.been.calledOnceWithExactly();
      expect(container.kill).to.not.have.been.called();
      expect(container.remove).to.have.been.calledOnceWithExactly();
      expect(container.remove).to.have.been.calledAfter(container.stop);
    });

    it('should do nothing when Core is not running', async () => {
      container.inspect.resolves({ State: { Status: 'exited' } });

      const result = await coreService.stop();

      expect(result).to.be.false();
      expect(container.stop).to.not.have.been.called();
    });
  });
});
