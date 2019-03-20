/**
 * @param sinonSandbox
 * @return {{fetchDPContract: function(id:string) : Promise<DPContract|null>,
 *       fetchTransaction: function(id:string) : Promise<{confirmations: number}>,
 *       fetchDocuments: function(dpContractId:string, type:string, where: Object) :
 *       Promise<Document[]>}}
 */
module.exports = function createDataProviderMock(sinonSandbox) {
  return {
    fetchDPContract: sinonSandbox.stub(),
    fetchDocuments: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
  };
};
