const Long = require('long');

const Identifier = require('@dashevo/dpp/lib/Identifier');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const getFeatureFlagForHeightFactory = require('../../../lib/featureFlag/getFeatureFlagForHeightFactory');

describe('getFeatureFlagForHeightFactory', () => {
  let featureFlagDataContractId;
  let fetchDocumentsMock;
  let getFeatureFlagForHeight;
  let document;

  beforeEach(function beforeEach() {
    featureFlagDataContractId = Identifier.from(Buffer.alloc(32, 1));

    ([document] = getDocumentsFixture());

    fetchDocumentsMock = this.sinon.stub();
    fetchDocumentsMock.resolves([
      document,
    ]);

    getFeatureFlagForHeight = getFeatureFlagForHeightFactory(
      featureFlagDataContractId,
      fetchDocumentsMock,
    );
  });

  it('should call `fetchDocuments` and return first item from the result', async () => {
    const result = await getFeatureFlagForHeight('someType', new Long(42));

    const query = {
      where: [
        ['enableAtHeight', '==', 42],
      ],
    };

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      featureFlagDataContractId,
      'someType',
      query,
      undefined,
    );
    expect(result).to.deep.equal(document);
  });
});
