const { InvalidStorageAdapter } = require('../../errors');

module.exports = async function configureAdapter(argAdapter) {
  let adapter;
  if (!argAdapter) throw new Error('Expected an adapter to configure');
  const argAdapterConstructorName = argAdapter.constructor.name;

  // In case of an adapter being a function, we assume it being a class non instantiated
  if (argAdapterConstructorName === 'Function') {
    // eslint-disable-next-line new-cap
    adapter = new argAdapter();
    if (adapter.config) {
      try {
        await adapter.config({ name: 'dashevo-wallet-lib' });
      } catch (e) {
        throw new Error(`Tried to config the adapter. Failed with reason ${e.message}`);
      }
    } else if (adapter.createInstance) await adapter.createInstance({ name: 'dashevo-wallet-lib' });
  } else if (argAdapterConstructorName === 'Object') {
    if (argAdapter.createInstance) throw new Error('Adapter instance not created');
    adapter = argAdapter;
  } else {
    // Instance of specific class
    adapter = argAdapter;
  }
  // Testing the storage
  if (!adapter.getItem || !adapter.setItem) {
    throw new InvalidStorageAdapter('expected getItem/setItem methods');
  }
  try {
    await adapter.getItem('dummy');
  } catch (e) {
    throw new InvalidStorageAdapter(e.message);
  }
  return adapter;
};
