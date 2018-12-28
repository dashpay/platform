const getDapObjectsFixture = require('../../../../lib/test/fixtures/getDapObjectsFixture');
const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');

const fetchDapObjectsByObjectsFactory = require('../../../../lib/stPacket/verification/fetchDapObjectsByObjectsFactory');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

describe('fetchDapObjectsByObjects', () => {
  let fetchDapObjectsByObjects;
  let dataProviderMock;
  let dapObjects;
  let dapContract;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    fetchDapObjectsByObjects = fetchDapObjectsByObjectsFactory(dataProviderMock);

    dapObjects = getDapObjectsFixture();
    dapContract = getDapContractFixture();
  });

  it('should fetch specified DAP Objects using DataProvider', async () => {
    dataProviderMock.fetchDapObjects.withArgs(
      dapContract.getId(),
      dapObjects[0].getType(),
    ).resolves([dapObjects[0]]);

    dataProviderMock.fetchDapObjects.withArgs(
      dapContract.getId(),
      dapObjects[1].getType(),
    ).resolves([dapObjects[1], dapObjects[2]]);

    const fetchedDapObjects = await fetchDapObjectsByObjects(dapContract.getId(), dapObjects);

    expect(dataProviderMock.fetchDapObjects).to.be.calledTwice();

    let where = { id: { $in: [dapObjects[0].getId()] } };

    expect(dataProviderMock.fetchDapObjects).to.be.calledWith(
      dapContract.getId(),
      dapObjects[0].getType(),
      { where },
    );

    where = { id: { $in: [dapObjects[1].getId(), dapObjects[2].getId()] } };

    expect(dataProviderMock.fetchDapObjects).to.be.calledWith(
      dapContract.getId(),
      dapObjects[1].getType(),
      { where },
    );

    expect(fetchedDapObjects).to.be.deep.equal(dapObjects);
  });
});
