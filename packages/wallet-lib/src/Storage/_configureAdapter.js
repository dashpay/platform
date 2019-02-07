const { InvalidStorageAdapter } = require('../errors');

module.exports = async function configureAdapter(argAdapter) {
  let adapter;
  if (!argAdapter) throw new Error('Expected an adapter to configure');
  const argAdapterContructorName = argAdapter.constructor.name;

  // In case of an adapter being a function, we assume it being a class non instanciated
  if (argAdapterContructorName === 'Function') {
    // eslint-disable-next-line new-cap
    adapter = new argAdapter();
    if (adapter.config) {
      try {
        await adapter.config({ name: 'dashevo-wallet-lib' });
      } catch (e) {
        console.error('Tried to config the adapter. Failed', e.message);
      }
    } else if (adapter.createInstance) await adapter.createInstance({ name: 'dashevo-wallet-lib' });
  } else if (argAdapterContructorName === 'Object') {
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
