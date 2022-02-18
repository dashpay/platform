const Identifier = require('@dashevo/dpp/lib/Identifier');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const { expect } = require('chai');

const Long = require('long');

const getLatestFeatureFlagFactory = require('../../../lib/featureFlag/getLatestFeatureFlagFactory');

describe('getLatestFeatureFlagFactory', () => {
  let featureFlagDataContractId;
  let fetchDocumentsMock;
  let getLatestFeatureFlag;
  let document;

  beforeEach(function beforeEach() {
    featureFlagDataContractId = Identifier.from(Buffer.alloc(32, 1));

    ([document] = getDocumentsFixture());

    fetchDocumentsMock = this.sinon.stub();
    fetchDocumentsMock.resolves([
      document,
    ]);

    getLatestFeatureFlag = getLatestFeatureFlagFactory(
      featureFlagDataContractId,
      fetchDocumentsMock,
    );
  });

  it('should call `fetchDocuments` and return first item from the result', async () => {
    const result = await getLatestFeatureFlag('someType', new Long(42));

    const query = {
      where: [
        ['enableAtHeight', '<=', 42],
      ],
      orderBy: [
        ['enableAtHeight', 'desc'],
      ],
      limit: 1,
    };

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      featureFlagDataContractId,
      'someType',
      query,
      false,
    );
    expect(result).to.deep.equal(document);
  });
});
