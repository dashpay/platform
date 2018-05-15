const startDashDriveInstanceMocha = require('../../../../../lib/test/services/mocha/startDashDriveInstance');

describe('startDashDriveInstance', () => {
  describe('One instance', () => {
    let instance;
    startDashDriveInstanceMocha().then((_instance) => {
      instance = _instance;
    });

    it('should has DashCore container running', async () => {
      const { State } = await instance.dashCore.container.details();
      expect(State.Status).to.equal('running');
    });

    it('should has MongoDb container running', async () => {
      const { State } = await instance.mongoDb.container.details();
      expect(State.Status).to.equal('running');
    });

    it('should has DashDrive container running', async () => {
      const { State } = await instance.dashDrive.container.details();
      expect(State.Status).to.equal('running');
    });
  });

  describe('Three instance', () => {
    let instances;
    startDashDriveInstanceMocha.many(3).then((_instance) => {
      instances = _instance;
    });

    it('should have DashCore containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].dashCore.container.details();
        expect(State.Status).to.equal('running');
      }
    });

    it('should have MongoDb containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].mongoDb.container.details();
        expect(State.Status).to.equal('running');
      }
    });

    it('should have DashDrive containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].dashDrive.container.details();
        expect(State.Status).to.equal('running');
      }
    });
  });
});

