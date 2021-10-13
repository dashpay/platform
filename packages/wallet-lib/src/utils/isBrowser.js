// eslint-disable-next-line no-new-func
const isBrowser = new Function('try {return this===window;}catch(e){ return false;}');

module.exports = isBrowser;
