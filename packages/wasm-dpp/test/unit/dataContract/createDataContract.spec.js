const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifierAsync');

const { default: loadWasmDpp } = require('../../..');

describe.skip('createDataContract', () => {
  let rawDataContract;
  let DataContract;

  before(async () => {
    ({
      DataContract,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    rawDataContract = {
      $id: (await generateRandomIdentifier()).toBuffer(),
      ownerId: (await generateRandomIdentifier()).toBuffer(),
      protocolVersion: 4,
      version: 20,
      $schema: 'http://test.com/schema',
      documents: {
        niceDocument: {
          name: { type: 'string' },
        },
      },
    };
  });

  it('should return new DataContract with "dataContractId" and documents', () => {
    const dataContract = new DataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getOwnerId().toBuffer()).to.deep.equal(rawDataContract.ownerId);
    expect(dataContract.getDocuments()).to.deep.equal(rawDataContract.documents);
  });

  it('should return new DataContract with "$schema" if present', () => {
    rawDataContract.$schema = 'http://test.com/otherschema';

    const dataContract = new DataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getJsonMetaSchema()).to.equal(rawDataContract.$schema);

    expect(dataContract.getOwnerId().toBuffer()).to.deep.equal(rawDataContract.ownerId);
    expect(dataContract.getDocuments()).to.deep.equal(rawDataContract.documents);
  });

  it('should return new DataContract with "$defs" if present', () => {
    rawDataContract.$defs = {
      subSchema: { type: 'object' },
    };

    const dataContract = new DataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getDefinitions()).to.deep.equal(rawDataContract.$defs);

    expect(dataContract.getOwnerId().toBuffer()).to.deep.equal(rawDataContract.ownerId);
    expect(dataContract.getDocuments()).to.deep.equal(rawDataContract.documents);
  });
});
