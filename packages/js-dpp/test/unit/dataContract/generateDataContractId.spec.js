const bs58 = require('bs58');
const generateDataContractId = require('../../../lib/dataContract/generateDataContractId');

describe('generateDataContractId', () => {
  let ownerId;
  let entropy;

  beforeEach(() => {
    ownerId = bs58.decode('23wdhodag');
    entropy = bs58.decode('5dz916pTe1');
  });

  it('should generate bs58 id based on ', () => {
    const id = bs58.decode('DJftzP4Db8pdN76p72SScmw27jcVjcxrqbT4mmZNMJcw');
    const generatedId = generateDataContractId(ownerId, entropy);

    expect(Buffer.compare(id, generatedId)).to.equal(0);
  });
});
