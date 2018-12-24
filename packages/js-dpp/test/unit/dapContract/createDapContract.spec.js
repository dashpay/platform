const createDapContract = require('../../../lib/dapContract/createDapContract');
const DapContract = require('../../../lib/dapContract/DapContract');

describe('createDapContract', () => {
  let rawDapContract;
  beforeEach(() => {
    rawDapContract = {
      name: 'LovelyContract',
      dapObjectsDefinition: {
        niceObject: {
          name: { type: 'string' },
        },
      },
    };
  });

  it('should return new DAP Contract with "name" and dapObjectsDefinition', () => {
    const dapContract = createDapContract(rawDapContract);

    expect(dapContract).to.be.instanceOf(DapContract);

    expect(dapContract.getName()).to.be.equal(rawDapContract.name);
    expect(dapContract.getDapObjectsDefinition()).to.be.equal(rawDapContract.dapObjectsDefinition);
  });

  it('should return new DAP Contract with "$schema" if present', () => {
    rawDapContract.$schema = 'http://test.com/schema';

    const dapContract = createDapContract(rawDapContract);

    expect(dapContract).to.be.instanceOf(DapContract);

    expect(dapContract.getJsonMetaSchema()).to.be.equal(rawDapContract.$schema);

    expect(dapContract.getName()).to.be.equal(rawDapContract.name);
    expect(dapContract.getDapObjectsDefinition()).to.be.equal(rawDapContract.dapObjectsDefinition);
  });

  it('should return new DAP Contract with "version" if present', () => {
    rawDapContract.version = 1;

    const dapContract = createDapContract(rawDapContract);

    expect(dapContract).to.be.instanceOf(DapContract);

    expect(dapContract.getVersion()).to.be.equal(rawDapContract.version);

    expect(dapContract.getName()).to.be.equal(rawDapContract.name);
    expect(dapContract.getDapObjectsDefinition()).to.be.equal(rawDapContract.dapObjectsDefinition);
  });

  it('should return new DAP Contract with "definitions" if present', () => {
    rawDapContract.definitions = {
      subSchema: { type: 'object' },
    };

    const dapContract = createDapContract(rawDapContract);

    expect(dapContract).to.be.instanceOf(DapContract);

    expect(dapContract.getDefinitions()).to.be.equal(rawDapContract.definitions);

    expect(dapContract.getName()).to.be.equal(rawDapContract.name);
    expect(dapContract.getDapObjectsDefinition()).to.be.equal(rawDapContract.dapObjectsDefinition);
  });
});
