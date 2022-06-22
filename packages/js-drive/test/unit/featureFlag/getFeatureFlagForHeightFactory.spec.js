const Long = require('long');

const Identifier = require('@dashevo/dpp/lib/Identifier');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const getFeatureFlagForHeightFactory = require('../../../lib/featureFlag/getFeatureFlagForHeightFactory');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('getFeatureFlagForHeightFactory', () => {
  let featureFlagDataContractId;
  let fetchDocumentsMock;
  let getFeatureFlagForHeight;
  let document;
  let featureFlagDataContractBlockHeight;
  let dataContract;
  let fetchDataContractMock;

  beforeEach(function beforeEach() {
    featureFlagDataContractId = Identifier.from(Buffer.alloc(32, 1));

    ([document] = getDocumentsFixture());

    dataContract = getDataContractFixture();

    fetchDataContractMock = this.sinon.stub().resolves(
      new StorageResult(dataContract),
    );
    fetchDocumentsMock = this.sinon.stub().resolves(
      new StorageResult([document]),
    );

    featureFlagDataContractBlockHeight = 42;

    getFeatureFlagForHeight = getFeatureFlagForHeightFactory(
      featureFlagDataContractId,
      fetchDocumentsMock,
      fetchDataContractMock,
    );
  });

  it('should call `fetchDocuments` and return first item from the result', async () => {
    const result = await getFeatureFlagForHeight('someType', new Long(43));

    const query = {
      where: [
        ['enableAtHeight', '==', 43],
      ],
    };

    expect(fetchDataContractMock).to.have.been.calledOnceWithExactly(
      featureFlagDataContractId,
      'someType',
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      new StorageResult(dataContract),
      'someType',
      {
        ...query,
        useTransaction: false,
      },
    );
    expect(result).to.deep.equal(document);
  });

  it('should return null if featureFlagDataContractId is undefined', async () => {
    getFeatureFlagForHeight = getFeatureFlagForHeightFactory(
      undefined,
      featureFlagDataContractBlockHeight,
      fetchDocumentsMock,
    );

    const result = await getFeatureFlagForHeight('someType', new Long(42));

    expect(result).to.equal(null);
    expect(fetchDocumentsMock).to.not.be.called();
  });
});
