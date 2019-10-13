const createContract = require('../../../lib/contract/createContract');
const Contract = require('../../../lib/contract/Contract');

describe('createContract', () => {
  let rawContract;
  beforeEach(() => {
    rawContract = {
      id: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      documents: {
        niceDocument: {
          name: { type: 'string' },
        },
      },
    };
  });

  it('should return new Contract with "contractId" and documents', () => {
    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getId()).to.equal(rawContract.contractId);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });

  it('should return new Contract with "$schema" if present', () => {
    rawContract.$schema = 'http://test.com/schema';

    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getJsonMetaSchema()).to.equal(rawContract.$schema);

    expect(contract.getId()).to.equal(rawContract.contractId);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });

  it('should return new Contract with "version" if present', () => {
    rawContract.version = 1;

    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getVersion()).to.equal(rawContract.version);

    expect(contract.getId()).to.equal(rawContract.contractId);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });

  it('should return new Contract with "definitions" if present', () => {
    rawContract.definitions = {
      subSchema: { type: 'object' },
    };

    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getDefinitions()).to.equal(rawContract.definitions);

    expect(contract.getId()).to.equal(rawContract.contractId);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });
});
