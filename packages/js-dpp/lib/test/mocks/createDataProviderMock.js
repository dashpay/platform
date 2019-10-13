/**
 * @param sinonSandbox
 * @return {{fetchContract: function(contractId:string) : Promise<Contract|null>,
 *       fetchTransaction: function(contractId:string) : Promise<{confirmations: number}>,
 *       fetchDocuments: function(contractId:string, type:string, where: Object) :
 *       Promise<Document[]>}}
 */
module.exports = function createDataProviderMock(sinonSandbox) {
  return {
    fetchContract: sinonSandbox.stub(),
    fetchDocuments: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
  };
};
