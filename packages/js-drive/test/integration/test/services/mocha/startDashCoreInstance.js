const startDashCoreInstance = require('../../../../../lib/test/services/mocha/startDashCoreInstance');

describe('startDashCoreInstance', () => {
  describe('One instance', () => {
    let instance;
    startDashCoreInstance().then((_instance) => {
      instance = _instance;
    });

    it('should has container running', async () => {
      const { State } = await instance.container.details();
      expect(State.Status).to.equal('running');
    });
  });

  describe('Three instances', () => {
    let instances;
    startDashCoreInstance.many(3).then((_instances) => {
      instances = _instances;
    });

    it('should have containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].container.details();
        expect(State.Status).to.equal('running');
      }
    });
  });
});
