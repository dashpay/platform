const register = require('../src/register');

register()
  // eslint-disable-next-line no-console
  .then(result => console.dir(result));
