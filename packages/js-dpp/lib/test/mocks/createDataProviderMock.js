/**
 * @param sinonSandbox
 * @return {{fetchDPContract: function(id:string) : Promise<DPContract|null>,
 *       fetchTransaction: function(id:string) : Promise<{confirmations: number}>,
 *       fetchDPObjects: function(dpContractId:string, type:string, where: Object) :
 *       Promise<DPObject[]>}}
 */
module.exports = function createDataProviderMock(sinonSandbox) {
  return {
    fetchDPContract: sinonSandbox.stub(),
    fetchDPObjects: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
  };
};
