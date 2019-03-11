const bs58 = require('bs58');

const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');
const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

const fetchDPObjectsByObjectsFactory = require('../../../../lib/stPacket/verification/fetchDPObjectsByObjectsFactory');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

function encodeToBase58(id) {
  const idBuffer = Buffer.from(id, 'hex');
  return bs58.encode(idBuffer);
}

describe('fetchDPObjectsByObjects', () => {
  let fetchDPObjectsByObjects;
  let dataProviderMock;
  let dpObjects;
  let dpContract;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    fetchDPObjectsByObjects = fetchDPObjectsByObjectsFactory(dataProviderMock);

    dpObjects = getDPObjectsFixture();
    dpContract = getDPContractFixture();
  });

  it('should fetch specified DP Objects using DataProvider', async () => {
    dataProviderMock.fetchDPObjects.withArgs(
      dpContract.getId(),
      dpObjects[0].getType(),
    ).resolves([dpObjects[0]]);

    dataProviderMock.fetchDPObjects.withArgs(
      dpContract.getId(),
      dpObjects[1].getType(),
    ).resolves([dpObjects[1], dpObjects[2]]);

    const fetchedDPObjects = await fetchDPObjectsByObjects(dpContract.getId(), dpObjects);

    expect(dataProviderMock.fetchDPObjects).to.have.been.calledTwice();

    let where = {
      _id: {
        $in: [
          encodeToBase58(dpObjects[0].getId()),
        ],
      },
    };

    expect(dataProviderMock.fetchDPObjects).to.have.been.calledWith(
      dpContract.getId(),
      dpObjects[0].getType(),
      { where },
    );

    where = {
      _id: {
        $in: [
          encodeToBase58(dpObjects[1].getId()),
          encodeToBase58(dpObjects[2].getId()),
        ],
      },
    };

    expect(dataProviderMock.fetchDPObjects).to.have.been.calledWith(
      dpContract.getId(),
      dpObjects[1].getType(),
      { where },
    );

    expect(fetchedDPObjects).to.deep.equal(dpObjects);
  });
});
