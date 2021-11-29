const validateIndicesAreNotChanged = require('../../../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/validation/basic/validateIndicesAreNotChanged');
const DataContractIndicesChangedError = require('../../../../../../../lib/errors/consensus/basic/dataContract/DataContractIndicesChangedError');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

describe('validateIndicesAreNotChanged', () => {
  let oldDocumentsSchema;
  let newDocumentsSchema;

  beforeEach(() => {
    const oldDataContract = getDataContractFixture();
    const newDataContract = getDataContractFixture();

    oldDocumentsSchema = oldDataContract.getDocuments();
    newDocumentsSchema = newDataContract.getDocuments();
  });

  it('should return invalid result if some index have changed', async () => {
    newDocumentsSchema.indexedDocument.indices[0].properties[0].$ownerId = 'desc';

    const result = await validateIndicesAreNotChanged(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractIndicesChangedError);
  });

  it('should return invalid result if index have been added', async () => {
    newDocumentsSchema.newDoc = {
      type: 'object',
      indices: {},
    };

    const result = await validateIndicesAreNotChanged(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractIndicesChangedError);
  });

  it('should return valid result if indicies are not changed', async () => {
    const result = await validateIndicesAreNotChanged(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.true();
  });
});
