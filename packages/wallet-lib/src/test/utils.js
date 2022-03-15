const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

module.exports = {
  sleepOneTick,
};
