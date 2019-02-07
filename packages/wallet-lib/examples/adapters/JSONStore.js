const JSONStorage = {
  createInstance: () => ({
    setItem: (key, item) => console.log('key', key, item),
    getItem: key => console.log(key),
  }),
};
module.exports = JSONStorage;
