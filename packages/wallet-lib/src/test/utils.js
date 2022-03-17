const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

const sleep = (timeout) => new Promise(((resolve) => setTimeout(resolve, timeout)));

module.exports = {
  sleepOneTick,
  sleep,
};
