const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../../../../../dist');

describe.skip('validateIndicesAreBackwardCompatible', () => {
  let oldDocumentsSchema;
  let newDocumentsSchema;
  let validateIndicesAreBackwardCompatible;
  let DataContractUniqueIndicesChangedError;
  let DataContractInvalidIndexDefinitionUpdateError;
  let DataContractHaveNewUniqueIndexError;

  before(async () => {
    ({
      validateIndicesAreBackwardCompatible,
      DataContractUniqueIndicesChangedError,
      DataContractInvalidIndexDefinitionUpdateError,
      DataContractHaveNewUniqueIndexError,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    const oldDataContract = await getDataContractFixture();
    const newDataContract = await getDataContractFixture();

    oldDocumentsSchema = oldDataContract.getDocuments();
    newDocumentsSchema = newDataContract.getDocuments();
  });

  it('should return invalid result if some of unique indices have changed', async () => {
    newDocumentsSchema.indexedDocument.indices[0].properties[1].firstName = 'desc';

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractUniqueIndicesChangedError);
    expect(error.getIndexName()).to.equal(newDocumentsSchema.indexedDocument.indices[0].name);
  });

  it('should return invalid result if already defined properties are changed in existing index', async () => {
    newDocumentsSchema.indexedDocument.indices[2].properties[0].lastName = 'desc';

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractInvalidIndexDefinitionUpdateError);
    expect(error.getIndexName()).to.equal(newDocumentsSchema.indexedDocument.indices[2].name);
  });

  it('should return invalid result if already indexed properties are added to existing index', async () => {
    newDocumentsSchema.indexedDocument.indices[2].properties.push({ firstName: 'asc' });

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataContractInvalidIndexDefinitionUpdateError);
    expect(error.getIndexName()).to.equal(newDocumentsSchema.indexedDocument.indices[2].name);
  });

  it('should return invalid result if one of new indices contains old properties in the wrong order', async () => {
    newDocumentsSchema.indexedDocument.indices.push({
      name: 'index_other',
      properties: [
        { firstName: 'asc' },
        { $ownerId: 'asc' },
      ],
    });

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    // TODO
    // expect(error).to.be.an.instanceOf(DataContractHaveNewIndexWithOldPropertiesError);
    expect(error.getIndexName()).to.equal('index_other');
  });

  it('should return invalid result if one of new indices is unique', async () => {
    newDocumentsSchema.indexedDocument.indices.push({
      name: 'index_other',
      properties: [
        { otherName: 'asc' },
      ],
      unique: true,
    });

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const error = result.getErrors()[0];

    expect(error).to.be.an.instanceOf(DataContractHaveNewUniqueIndexError);
    expect(error.getIndexName()).to.equal('index_other');
  });

  it('should return invalid result if existing property was used in a new index', async () => {
    newDocumentsSchema.indexedDocument.indices.push({
      name: 'oldFieldIndex',
      properties: [
        {
          otherProperty: 'asc',
        },
      ],
    });

    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.false();

    const [error] = result.getErrors();
    expect(error).to.be.an.instanceOf(DataContractInvalidIndexDefinitionUpdateError);
    expect(error.getIndexName()).to.equal('oldFieldIndex');
  });

  it('should return valid result if indices are not changed', async () => {
    const result = validateIndicesAreBackwardCompatible(oldDocumentsSchema, newDocumentsSchema);

    expect(result.isValid()).to.be.true();
  });
});
