const createDPContract = require('../../../lib/contract/createDPContract');
const DPContract = require('../../../lib/contract/DPContract');

describe('createDPContract', () => {
  let rawDPContract;
  beforeEach(() => {
    rawDPContract = {
      name: 'LovelyContract',
      dpObjectsDefinition: {
        niceObject: {
          name: { type: 'string' },
        },
      },
    };
  });

  it('should return new DP Contract with "name" and dpObjectsDefinition', () => {
    const dpContract = createDPContract(rawDPContract);

    expect(dpContract).to.be.instanceOf(DPContract);

    expect(dpContract.getName()).to.be.equal(rawDPContract.name);
    expect(dpContract.getDPObjectsDefinition()).to.be.equal(rawDPContract.dpObjectsDefinition);
  });

  it('should return new DP Contract with "$schema" if present', () => {
    rawDPContract.$schema = 'http://test.com/schema';

    const dpContract = createDPContract(rawDPContract);

    expect(dpContract).to.be.instanceOf(DPContract);

    expect(dpContract.getJsonMetaSchema()).to.be.equal(rawDPContract.$schema);

    expect(dpContract.getName()).to.be.equal(rawDPContract.name);
    expect(dpContract.getDPObjectsDefinition()).to.be.equal(rawDPContract.dpObjectsDefinition);
  });

  it('should return new DP Contract with "version" if present', () => {
    rawDPContract.version = 1;

    const dpContract = createDPContract(rawDPContract);

    expect(dpContract).to.be.instanceOf(DPContract);

    expect(dpContract.getVersion()).to.be.equal(rawDPContract.version);

    expect(dpContract.getName()).to.be.equal(rawDPContract.name);
    expect(dpContract.getDPObjectsDefinition()).to.be.equal(rawDPContract.dpObjectsDefinition);
  });

  it('should return new DP Contract with "definitions" if present', () => {
    rawDPContract.definitions = {
      subSchema: { type: 'object' },
    };

    const dpContract = createDPContract(rawDPContract);

    expect(dpContract).to.be.instanceOf(DPContract);

    expect(dpContract.getDefinitions()).to.be.equal(rawDPContract.definitions);

    expect(dpContract.getName()).to.be.equal(rawDPContract.name);
    expect(dpContract.getDPObjectsDefinition()).to.be.equal(rawDPContract.dpObjectsDefinition);
  });
});
