const validateIndicesAreBackwardCompatible = require('../../../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/validation/basic/validateIndicesAreBackwardCompatible');
const DataContractHaveNewIndexWithOldPropertiesError = require('../../../../../../../lib/errors/consensus/basic/dataContract/DataContractInvalidIndexDefinitionUpdateError');
const DataContractHaveNewUniqueIndexError = require('../../../../../../../lib/errors/consensus/basic/dataContract/DataContractHaveNewUniqueIndexError');
const DataContractIndicesChangedError = require('../../../../../../../lib/errors/consensus/basic/dataContract/DataContractUniqueIndicesChangedError');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const DataContractInvalidIndexDefinitionUpdateError = require('../../../../../../../lib/errors/consensus/basic/dataContract/DataContractInvalidIndexDefinitionUpdateError');

describe('validateIndicesAreBackwardCompatible', () => {
  let oldDocumentsSchema;
  let newDocumentsSchema;

  beforeEach(() => {
    const oldDataContract = getDataContractFixture();
    const newDataContract = getDataContractFixture();

    newDataContract.getDocumentSchema('indexedDocument').properties.otherName = {
      type: 'string',
    };

    newDataContract.getDocumentSchema('indexedDocument').indices.push({
      name: 'index42',
      properties: [
        { otherName: 'desc' },
      ],
    });

    oldDocumentsSchema = oldDataContract.getDocuments();
    newDocumentsSchema = newDataContract.getDocuments();
  });

  it('should return invalid result if some of unique indices have changed', async () => {
    newDocumentsSchema.indexedDocument.indices[0].properties[0].$ownerId = 'desc';

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractIndicesChangedError);
    expect(error.getIndexName()).to.equal(newDocumentsSchema.indexedDocument.indices[0].name);
  });

  it('should return invalid result if non-unique index update failed due to changed old properties', async () => {
    newDocumentsSchema.indexedDocument.indices[2].properties[0].$id = 'desc';

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractInvalidIndexDefinitionUpdateError);
    expect(error.getIndexName()).to.equal(newDocumentsSchema.indexedDocument.indices[2].name);
  });

  it('should return invalid result if non-unique index update failed due old properties used', async () => {
    newDocumentsSchema.indexedDocument.indices[2].properties.push({ firstName: 'asc' });

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractInvalidIndexDefinitionUpdateError);
    expect(error.getIndexName()).to.equal(newDocumentsSchema.indexedDocument.indices[2].name);
  });

  it('should return invalid result if one of new indices contains old properties in the wrong order', async () => {
    newDocumentsSchema.indexedDocument.indices.push({
      name: 'index_other',
      properties: [
        { firstName: 'desc' },
        { $ownerId: 'asc' },
      ],
    });

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractHaveNewIndexWithOldPropertiesError);
    expect(error.getIndexName()).to.equal('index_other');
  });

  it('should return invalid result if one of new indices is unique', async () => {
    newDocumentsSchema.indexedDocument.indices.push({
      name: 'index_other',
      properties: [
        { otherName: 'desc' },
      ],
      unique: true,
    });

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractHaveNewUniqueIndexError);
    expect(error.getIndexName()).to.equal('index_other');
  });

  it('should return valid result if indicies are not changed', async () => {
    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.true();
  });
});
