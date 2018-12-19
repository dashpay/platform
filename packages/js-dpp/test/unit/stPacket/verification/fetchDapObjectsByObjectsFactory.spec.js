const getDapObjectsFixture = require('../../../../lib/test/fixtures/getDapObjectsFixture');
const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');

const AbstractDataProvider = require('../../../../lib/dataProvider/AbstractDataProvider');

const fetchDapObjectsByObjectsFactory = require('../../../../lib/stPacket/verification/fetchDapObjectsByObjectsFactory');

describe('fetchDapObjectsByObjects', () => {
  let fetchDapObjectsByObjects;
  let fetchDapObjectsMock;
  let dapObjects;
  let dapContract;

  beforeEach(function beforeEach() {
    const dataProviderMock = this.sinonSandbox.createStubInstance(AbstractDataProvider, {
      fetchDapObjects: this.sinonSandbox.stub(),
    });

    fetchDapObjectsMock = dataProviderMock.fetchDapObjects;

    fetchDapObjectsByObjects = fetchDapObjectsByObjectsFactory(dataProviderMock);

    dapObjects = getDapObjectsFixture();
    dapContract = getDapContractFixture();
  });

  it('should fetch specified DAP Objects using DataProvider', async () => {
    fetchDapObjectsMock.withArgs(
      dapContract.getId(),
      dapObjects[0].getType(),
    ).resolves([dapObjects[0]]);

    fetchDapObjectsMock.withArgs(
      dapContract.getId(),
      dapObjects[1].getType(),
    ).resolves([dapObjects[1], dapObjects[2]]);

    const fetchedDapObjects = await fetchDapObjectsByObjects(dapContract.getId(), dapObjects);

    expect(fetchDapObjectsMock).to.be.calledTwice();

    let where = { id: { $in: [dapObjects[0].getId()] } };

    expect(fetchDapObjectsMock).to.be.calledWith(
      dapContract.getId(),
      dapObjects[0].getType(),
      { where },
    );

    where = { id: { $in: [dapObjects[1].getId(), dapObjects[2].getId()] } };

    expect(fetchDapObjectsMock).to.be.calledWith(
      dapContract.getId(),
      dapObjects[1].getType(),
      { where },
    );

    expect(fetchedDapObjects).to.be.deep.equal(dapObjects);
  });
});
