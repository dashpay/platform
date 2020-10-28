const { startMongoDb, startDashCore } = require('@dashevo/dp-services-ctl');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('detectStandaloneRegtestModeFactory', function main() {
  this.timeout(90000);

  let mongoDB;
  let container;
  let firstDashCore;
  let secondDashCore;

  before(async () => {
    mongoDB = await startMongoDb();
  });

  after(async () => {
    await mongoDB.remove();
    if (firstDashCore) {
      await firstDashCore.remove();
    }

    if (secondDashCore) {
      await secondDashCore.remove();
    }
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should return true if chain is regtest and has no peers', async () => {
    firstDashCore = await startDashCore();

    container = await createTestDIContainer(mongoDB, firstDashCore);

    const detectStandaloneRegtestMode = container.resolve('detectStandaloneRegtestMode');

    const result = await detectStandaloneRegtestMode();

    expect(result).to.be.true();
  });

  it('should return false if peers count > 0', async () => {
    firstDashCore = await startDashCore();

    secondDashCore = await startDashCore();
    await secondDashCore.connect(firstDashCore);

    container = await createTestDIContainer(mongoDB, firstDashCore);

    const detectStandaloneRegtestMode = container.resolve('detectStandaloneRegtestMode');

    const result = await detectStandaloneRegtestMode();

    expect(result).to.be.false();
  });
});
