/**
 * @param sinonSandbox
 * @return {{fetchDapContract: function(id:string) : Promise<DapContract|null>,
 *       fetchTransaction: function(id:string) : Promise<{confirmations: number}>,
 *       fetchDapObjects: function(dapContractId:string, type:string, where: Object) :
 *       Promise<DapObject[]>}}
 */
module.exports = function createDataProviderMock(sinonSandbox) {
  return {
    fetchDapContract: sinonSandbox.stub(),
    fetchDapObjects: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
  };
};
