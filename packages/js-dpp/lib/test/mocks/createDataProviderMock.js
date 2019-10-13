/**
 * @param sinonSandbox
 * @return {{fetchDataContract: function(contractId:string) : Promise<DataContract|null>,
 *       fetchTransaction: function(contractId:string) : Promise<{confirmations: number}>,
 *       fetchDocuments: function(contractId:string, type:string, where: Object) :
 *       Promise<Document[]>}}
 */
module.exports = function createDataProviderMock(sinonSandbox) {
  return {
    fetchDataContract: sinonSandbox.stub(),
    fetchDocuments: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
  };
};
