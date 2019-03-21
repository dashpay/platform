const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const fetchDapObjectsFactory = require('../../../../lib/rpcServer/commands/fetchDapObjects');
const DashDriveAdapter = require('../../../../lib/externalApis/dashDriveAdapter');

const dashDriveAdapter = new DashDriveAdapter({ host: 'host', port: 1 });

chai.use(chaiAsPromised);
const { expect } = chai;

const expectedSearchParams = { contractId: '123', type: 'contact', options: { where: { userId: 1 } } };
const expectedResult = [{ contractId: '123', type: 'contact', userId: 1 }];

describe('fetchDapContract', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const fetchDapObjects = fetchDapObjectsFactory(dashDriveAdapter);
      expect(fetchDapObjects).to.be.a('function');
    });
  });

  before(() => {
    sinon.stub(dashDriveAdapter, 'fetchDapObjects')
      .withArgs(expectedSearchParams.contractId, expectedSearchParams.type,
        expectedSearchParams.options).returns(Promise.resolve(expectedResult));
  });

  beforeEach(() => {
    dashDriveAdapter.fetchDapObjects.resetHistory();
  });

  after(() => {
    dashDriveAdapter.fetchDapObjects.restore();
  });

  it('Should return dap objects', async () => {
    const fetchDapObjects = fetchDapObjectsFactory(dashDriveAdapter);
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    const dapObjects = await fetchDapObjects(expectedSearchParams);
    expect(dapObjects).to.be.equal(expectedResult);
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const fetchDapObjects = fetchDapObjectsFactory(dashDriveAdapter);
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects({ contractId: 123 })).to.be.rejectedWith('params.contractId should be string');
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects({ contractId: '123' })).to.be.rejectedWith('params should have required property \'type\'');
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects({ contractId: '123', type: 1 })).to.be.rejectedWith('params.type should be string');
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects({ contractId: '123', type: 'type' })).to.be.rejectedWith('params should have required property \'options\'');
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects({ contractId: '123', type: 'type', options: 1 })).to.be.rejectedWith('params.options should be object');
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects({})).to.be.rejectedWith('params should have required property \'contractId\'');
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects()).to.be.rejectedWith('params should be object');
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects([123])).to.be.rejected;
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
    await expect(fetchDapObjects([-1])).to.be.rejected;
    expect(dashDriveAdapter.fetchDapObjects.callCount).to.be.equal(0);
  });
});
