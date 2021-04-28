class LocalForageAdapterMock {
  constructor() {
    this.isConfig = false;
    this.keys = {};
  }

  config() {
    this.isConfig = true;
  }

  setItem(key, item) {
    this.keys[key] = JSON.stringify(item);
    return item;
  }

  getItem(key) {
    return this.keys[key] ? JSON.parse(this.keys[key]) : null;
  }
}
module.exports = LocalForageAdapterMock;
