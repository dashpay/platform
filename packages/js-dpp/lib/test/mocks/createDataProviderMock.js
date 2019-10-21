/**
 * @param sinonSandbox
 * @return {{fetchDataContract: *, fetchDocuments: *, fetchTransaction: *}}
 */
module.exports = function createDataProviderMock(sinonSandbox) {
  return {
    fetchDataContract: sinonSandbox.stub(),
    fetchDocuments: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
  };
};
