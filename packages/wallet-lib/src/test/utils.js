const waitOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

const wait = (timeout) => new Promise(((resolve) => setTimeout(resolve, timeout)));

module.exports = {
  waitOneTick,
  wait,
};
