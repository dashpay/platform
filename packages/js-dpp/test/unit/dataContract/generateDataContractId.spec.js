const generateDataContractId = require('../../../lib/dataContract/generateDataContractId');

describe('generateDataContractId', () => {
  let ownerId;
  let entropy;

  beforeEach(() => {
    ownerId = '23wdhodag';
    entropy = '5dz916pTe1';
  });

  it('should generate bs58 id based on ', () => {
    expect(generateDataContractId(ownerId, entropy)).to.equal(
      'CnS7cz4z1qoPsNfEgpgyVnKdtH2u7bgzZXHLcCQt24US',
    );
  });
});
