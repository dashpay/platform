const DataContract = require('../../../lib/dataContract/DataContract');

const generateRandomId = require('../../../lib/test/utils/generateRandomId');

describe('createDataContract', () => {
  let rawDataContract;

  beforeEach(() => {
    rawDataContract = {
      $id: generateRandomId().toBuffer(),
      ownerId: generateRandomId().toBuffer(),
      contractId: generateRandomId().toBuffer(),
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

    expect(dataContract.getOwnerId()).to.deep.equal(rawDataContract.ownerId);
    expect(dataContract.getDocuments()).to.equal(rawDataContract.documents);
  });

  it('should return new DataContract with "$schema" if present', () => {
    rawDataContract.$schema = 'http://test.com/schema';

    const dataContract = new DataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getJsonMetaSchema()).to.equal(rawDataContract.$schema);

    expect(dataContract.getOwnerId()).to.deep.equal(rawDataContract.ownerId);
    expect(dataContract.getDocuments()).to.equal(rawDataContract.documents);
  });

  it('should return new DataContract with "definitions" if present', () => {
    rawDataContract.definitions = {
      subSchema: { type: 'object' },
    };

    const dataContract = new DataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getDefinitions()).to.equal(rawDataContract.definitions);

    expect(dataContract.getOwnerId()).to.deep.equal(rawDataContract.ownerId);
    expect(dataContract.getDocuments()).to.equal(rawDataContract.documents);
  });
});
