const createDataContract = require('../../../lib/dataContract/createDataContract');
const DataContract = require('../../../lib/dataContract/DataContract');

describe('createDataContract', () => {
  let rawDataContract;

  beforeEach(() => {
    rawDataContract = {
      contractId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      documents: {
        niceDocument: {
          name: { type: 'string' },
        },
      },
    };
  });

  it('should return new DataContract with "dataContractId" and documents', () => {
    const dataContract = createDataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getId()).to.equal(rawDataContract.contractId);
    expect(dataContract.getDocuments()).to.equal(rawDataContract.documents);
  });

  it('should return new DataContract with "$schema" if present', () => {
    rawDataContract.$schema = 'http://test.com/schema';

    const dataContract = createDataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getJsonMetaSchema()).to.equal(rawDataContract.$schema);

    expect(dataContract.getId()).to.equal(rawDataContract.contractId);
    expect(dataContract.getDocuments()).to.equal(rawDataContract.documents);
  });

  it('should return new DataContract with "version" if present', () => {
    rawDataContract.version = 1;

    const dataContract = createDataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getVersion()).to.equal(rawDataContract.version);

    expect(dataContract.getId()).to.equal(rawDataContract.contractId);
    expect(dataContract.getDocuments()).to.equal(rawDataContract.documents);
  });

  it('should return new DataContract with "definitions" if present', () => {
    rawDataContract.definitions = {
      subSchema: { type: 'object' },
    };

    const dataContract = createDataContract(rawDataContract);

    expect(dataContract).to.be.an.instanceOf(DataContract);

    expect(dataContract.getDefinitions()).to.equal(rawDataContract.definitions);

    expect(dataContract.getId()).to.equal(rawDataContract.contractId);
    expect(dataContract.getDocuments()).to.equal(rawDataContract.documents);
  });
});
