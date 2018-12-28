/**
 * @param sinonSandbox
 * @return {{fetchDapContract: function(id:string):DapContract|null,
 *         fetchTransaction: function(id:string):{confirmations: number},
 *         fetchDapObjects: function(dapContractId:string, type:string, where: Object):DapObject[]}}
 */
module.exports = function createDataProviderMock(sinonSandbox) {
  return {
    fetchDapContract: sinonSandbox.stub(),
    fetchDapObjects: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
  };
};
