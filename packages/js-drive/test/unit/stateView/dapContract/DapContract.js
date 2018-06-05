const DapContract = require('../../../../lib/stateView/dapContract/DapContract');

describe('DapContract', () => {
  it('should serialize DapContract', () => {
    const dapId = '123456';
    const dapName = 'DashPay';
    const packetHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const schema = {};
    const dapContract = new DapContract(dapId, dapName, packetHash, schema);

    const dapContractSerialized = dapContract.toJSON();
    expect(dapContractSerialized).to.deep.equal({
      dapId,
      dapName,
      packetHash,
      schema,
    });
  });
});
