/**
 * Disconnect all the storage worker and process all account to disconnect their endpoint too.
 */
async function disconnect() {
  if (this.storage) {
    await this.storage.stopWorker();
  }
  if (this.accounts) {
    const accountPath = Object.keys(this.accounts);
    // eslint-disable-next-line guard-for-in,no-restricted-syntax
    for (const path in accountPath) {
      // eslint-disable-next-line no-await-in-loop
      await this.accounts[path].disconnect();
    }
  }
}
module.exports = disconnect;
