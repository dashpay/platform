/**
 * Disconnect all the storage worker and process all account to disconnect their endpoint too.
 */
function disconnect() {
  if (this.storage) {
    this.storage.stopWorker();
  }
  if (this.accounts) {
    const accountPath = Object.keys(this.accounts);
    accountPath.forEach((path) => {
      this.accounts[path].disconnect();
    });
  }
}
module.exports = disconnect;
