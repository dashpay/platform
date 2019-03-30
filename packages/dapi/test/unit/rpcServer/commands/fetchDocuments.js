const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const fetchDocumentsFactory = require('../../../../lib/rpcServer/commands/fetchDocuments');
const DriveAdapter = require('../../../../lib/externalApis/driveAdapter');

const driveAdapter = new DriveAdapter({ host: 'host', port: 1 });

chai.use(chaiAsPromised);
const { expect } = chai;

const expectedSearchParams = { contractId: '123', type: 'contact', options: { where: { userId: 1 } } };
const expectedResult = [{ contractId: '123', type: 'contact', userId: 1 }];

describe('fetchDocuments', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const fetchDocuments = fetchDocumentsFactory(driveAdapter);
      expect(fetchDocuments).to.be.a('function');
    });
  });

  before(() => {
    sinon.stub(driveAdapter, 'fetchDocuments')
      .withArgs(expectedSearchParams.contractId, expectedSearchParams.type,
        expectedSearchParams.options).returns(Promise.resolve(expectedResult));
  });

  beforeEach(() => {
    driveAdapter.fetchDocuments.resetHistory();
  });

  after(() => {
    driveAdapter.fetchDocuments.restore();
  });

  it('Should return dap objects', async () => {
    const fetchDocuments = fetchDocumentsFactory(driveAdapter);
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    const documents = await fetchDocuments(expectedSearchParams);
    expect(documents).to.be.equal(expectedResult);
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const fetchDocuments = fetchDocumentsFactory(driveAdapter);
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments({ contractId: 123 })).to.be.rejectedWith('params.contractId should be string');
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments({ contractId: '123' })).to.be.rejectedWith('params should have required property \'type\'');
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments({ contractId: '123', type: 1 })).to.be.rejectedWith('params.type should be string');
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments({ contractId: '123', type: 'type' })).to.be.rejectedWith('params should have required property \'options\'');
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments({ contractId: '123', type: 'type', options: 1 })).to.be.rejectedWith('params.options should be object');
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments({})).to.be.rejectedWith('params should have required property \'contractId\'');
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments()).to.be.rejectedWith('params should be object');
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments([123])).to.be.rejected;
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
    await expect(fetchDocuments([-1])).to.be.rejected;
    expect(driveAdapter.fetchDocuments.callCount).to.be.equal(0);
  });
});
