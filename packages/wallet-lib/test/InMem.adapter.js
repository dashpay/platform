class InMem {
  constructor() {
    this.isConfig = false;
    this.keys = {};
  }

  config() {
    this.isConfig = true;
  }

  setItem(key, item) {
    this.keys[key] = item;
    return item;
  }

  getItem(key) {
    return this.keys[key] || null;
  }
}
module.exports = InMem;
