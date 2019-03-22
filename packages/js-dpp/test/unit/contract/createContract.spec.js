const createContract = require('../../../lib/contract/createContract');
const Contract = require('../../../lib/contract/Contract');

describe('createContract', () => {
  let rawContract;
  beforeEach(() => {
    rawContract = {
      name: 'LovelyContract',
      documents: {
        niceDocument: {
          name: { type: 'string' },
        },
      },
    };
  });

  it('should return new Contract with "name" and documents', () => {
    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getName()).to.equal(rawContract.name);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });

  it('should return new Contract with "$schema" if present', () => {
    rawContract.$schema = 'http://test.com/schema';

    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getJsonMetaSchema()).to.equal(rawContract.$schema);

    expect(contract.getName()).to.equal(rawContract.name);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });

  it('should return new Contract with "version" if present', () => {
    rawContract.version = 1;

    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getVersion()).to.equal(rawContract.version);

    expect(contract.getName()).to.equal(rawContract.name);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });

  it('should return new Contract with "definitions" if present', () => {
    rawContract.definitions = {
      subSchema: { type: 'object' },
    };

    const contract = createContract(rawContract);

    expect(contract).to.be.an.instanceOf(Contract);

    expect(contract.getDefinitions()).to.equal(rawContract.definitions);

    expect(contract.getName()).to.equal(rawContract.name);
    expect(contract.getDocuments()).to.equal(rawContract.documents);
  });
});
