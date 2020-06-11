const dotenvSafe = require('dotenv-safe');

const path = require('path');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});
