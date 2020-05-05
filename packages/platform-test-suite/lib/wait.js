module.exports = function wait(ms) {
  return new Promise((res) => setTimeout(res, ms));
};
