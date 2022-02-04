const fs = require('fs');

/**
 * @param {GroveDBStore} groveDBStore
 * @param {GroveDBStore} signedGroveDBStore
 * @param {string} dbPath
 * @return {rotateSignedStore}
 */
function rotateSignedStoreFactory(groveDBStore, signedGroveDBStore, dbPath) {
  /**
   * @typedef {rotateSignedStore}
   * @param {Long} height
   * @returns {Promise<boolean>}
   */
  async function rotateSignedStore(height) {
    if (height.lessThanOrEqual(2)) {
      return false;
    }

    const signedStateHeight = height.subtract(2);
    const signedStatePath = `${dbPath}/signed_state_${signedStateHeight}`;

    const previousSignedStateHeight = signedStateHeight.subtract(1);
    const previousSignedStatePath = `${dbPath}/signed_state_${previousSignedStateHeight}`;

    const newSignedGroveDB = groveDBStore.checkpoint(signedStatePath);
    const previousSignedGroveDB = singedGroveDBStore.getDB();

    if (previousSignedStateHeight.greaterThan(0)) {
      await previousSignedGroveDB.close();

      fs.rmSync(previousSignedStatePath, { recursive: true });
    }

    singedGroveDBStore.setDB(newSignedGroveDB);

    return true;
  }

  module.exports = rotateSignedStore;
}

module.exports = rotateSignedStoreFactory;
