const { Isolate } = require('isolated-vm');

/**
 *
 * @param {ExternalCopy<ArrayBuffer>} snapshot
 * @param {Object} options
 * @param [options.timeout] - Timeout ms
 * @param [options.memoryLimit] - Memory limit mb
 * @return {Promise<{isolate: module:isolated-vm.Isolate, context: Context}>}
 */
async function bootstrapIsolateFromSnapshot(snapshot, options) {
  const isolate = new Isolate({
    ...options,
    snapshot,
  });

  const context = await isolate.createContext();
  const { global: jail } = context;

  await jail.set('global', jail.derefInto());

  return { context, isolate };
}

module.exports = bootstrapIsolateFromSnapshot;
